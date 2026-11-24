# fast-anything's R side. The CLI copies this file to <project>/.fast/runner.R and runs it as
#   Rscript runner.R <mode> <job.R>
# where job.R sets `job`. Modes: report (Rprof output -> profile.json), verify. Base R only; not
# meant to be run by hand.

args <- commandArgs(trailingOnly = TRUE)
mode <- args[[1]]
job <- local({ env <- new.env(); sys.source(args[[2]], envir = env); env$job })
here <- dirname(normalizePath(sub("^--file=", "", grep("^--file=", commandArgs(FALSE), value = TRUE)[1])))
fa <- new.env()
sys.source(file.path(here, "bridge.R"), envir = fa)

# ------------------------------------------------------------------ static analysis

SKIP_DIRS <- c(".fast", ".git", "renv", "packrat", "node_modules")
IO <- c("cat", "print", "message", "warning", "writeLines", "readLines", "readline", "readRDS", "saveRDS",
        "load", "save", "file", "url", "download.file", "read.csv", "read.table", "read.delim", "write.csv",
        "write.table", "sink", "system", "system2", "Sys.sleep", "scan", "source", "sys.source", "library",
        "require", "requireNamespace", "Sys.setenv", "setwd", "dir.create", "unlink", "file.remove")
NONDET <- c("runif", "rnorm", "rbinom", "rpois", "rexp", "sample", "sample.int", "set.seed", "Sys.time",
            "Sys.Date", "proc.time", "date", "Sys.getpid", "tempfile")
SELF <- c("missing", "sys.call", "sys.function", "match.call", "parent.frame", "sys.frame", "sys.frames",
          "environment", "substitute", "eval", "evalq", "match.arg", "on.exit", "nargs")
GUARDS <- c("sys.nframe() == 0", "sys.nframe() == 0L", "0 == sys.nframe()", "0L == sys.nframe()")

# names of the functions a piece of code calls
calls_in <- function(code) {
  out <- character()
  walk <- function(x) {
    if (is.call(x)) {
      f <- x[[1]]
      if (is.symbol(f)) out <<- c(out, as.character(f))
      if (is.call(f) && as.character(f[[1]]) %in% c("::", ":::")) out <<- c(out, as.character(f[[3]]))
      parts <- as.list(x)
      for (k in seq_along(parts)) {
        # an empty argument (m[, 1]) is the missing-argument symbol, which can't be passed on
        if (is.symbol(parts[[k]]) && !nzchar(as.character(parts[[k]]))) next
        walk(parts[[k]])
      }
    }
  }
  walk(code)
  unique(out)
}

is_definition <- function(e) {
  is.call(e) && as.character(e[[1]])[1] %in% c("<-", "=") && is.symbol(e[[2]]) &&
    is.call(e[[3]]) && identical(e[[3]][[1]], as.name("function"))
}

# every top-level function definition in a file: name -> details
describe_file <- function(path, rel, is_entry) {
  exprs <- tryCatch(parse(path, keep.source = TRUE, encoding = "UTF-8"), error = function(e) NULL)
  if (is.null(exprs)) return(list())
  refs <- attr(exprs, "srcref")
  guard <- NA
  for (i in seq_along(exprs)) {
    e <- exprs[[i]]
    if (is.call(e) && identical(e[[1]], as.name("if")) && paste(deparse(e[[2]]), collapse = "") %in% GUARDS) {
      guard <- refs[[i]][1]
      break
    }
  }
  top <- vapply(exprs, function(e) if (is_definition(e)) as.character(e[[2]]) else "", "")
  module <- if (is_entry) "__main__" else gsub("/", ".", sub("\\.[Rr]$", "", rel))
  out <- list()
  for (i in seq_along(exprs)) {
    e <- exprs[[i]]
    if (!is_definition(e)) next
    name <- as.character(e[[2]])
    fun <- e[[3]]
    params <- as.list(fun[[2]])
    used <- calls_in(fun[[3]])
    defaults <- lapply(seq_along(params), function(k) {
      d <- paste(deparse(params[[k]]), collapse = " ")  # "" for a parameter without a default
      if (nzchar(d)) d else NULL
    })
    blockers <- character()
    warnings <- character()
    if (is_entry && is.na(guard)) {
      blockers <- c(blockers, "lives in the entry script, which runs when sourced: put its main code under if (sys.nframe() == 0)")
    } else if (is_entry && refs[[i]][1] > guard) {
      blockers <- c(blockers, "defined inside or after the if (sys.nframe() == 0) block")
    }
    if ("..." %in% names(params)) blockers <- c(blockers, "takes ...")
    io <- intersect(used, IO)
    if (length(io)) blockers <- c(blockers, paste("does I/O:", paste(head(io, 3), collapse = ", ")))
    nondet <- intersect(used, NONDET)
    if (length(nondet)) blockers <- c(blockers, paste("not deterministic:", paste(head(nondet, 3), collapse = ", ")))
    if (any(c("<<-", "assign") %in% used)) blockers <- c(blockers, "writes global state")
    self <- intersect(used, SELF)
    if (length(self)) blockers <- c(blockers, paste("inspects its own call:", paste(head(self, 3), collapse = ", ")))
    helpers <- setdiff(intersect(used, top[nzchar(top)]), name)
    if (length(helpers)) warnings <- c(warnings, paste("calls", paste(head(helpers, 3), collapse = ", "), "- they get ported together with it"))
    warnings <- c(warnings, "R has no type annotations, types are inferred from recorded calls")
    ref <- refs[[i]]
    out[[length(out) + 1L]] <- list(
      qualname = name, module = module, kind = "function", owner = NULL,
      file = rel, line = ref[1], end_line = ref[3], name = name,
      params = unname(Map(function(n, d) list(name = n, annotation = NULL, default = d), names(params), defaults)),
      returns = NULL, blockers = as.list(blockers), warnings = as.list(warnings),
      portability = if (length(blockers)) 0 else 0.9, calls = as.list(helpers),
      source = paste(as.character(ref), collapse = "\n"))
  }
  out
}

project_files <- function(root) {
  files <- list.files(root, pattern = "\\.[Rr]$", recursive = TRUE, all.files = FALSE)
  files[!vapply(strsplit(files, "/", fixed = TRUE), function(p) any(head(p, -1) %in% SKIP_DIRS), TRUE)]
}

# ------------------------------------------------------------------ report

if (mode == "report") {
  wall <- as.numeric(readLines(paste0(job$rprof, ".wall")))
  times <- tryCatch(utils::summaryRprof(job$rprof)$by.total, error = function(e) NULL)
  functions <- list()
  if (!is.null(times) && nrow(times)) {
    hot <- gsub("\"", "", rownames(times), fixed = TRUE)
    entry <- if (nzchar(job$entry)) normalizePath(job$entry) else ""
    for (rel in project_files(job$root)) {
      path <- normalizePath(file.path(job$root, rel))
      for (def in describe_file(path, rel, identical(path, entry))) {
        row <- match(def$name, hot)
        if (is.na(row)) next
        def$ncalls <- 0
        def$tottime <- times$self.time[row]
        def$cumtime <- times$total.time[row]
        functions[[length(functions) + 1L]] <- def
      }
    }
  }
  writeLines(fa$.fa_json(list(wall = wall, base = ".", functions = functions)), job$out, useBytes = TRUE)
}

# ------------------------------------------------------------------ verify

seed <- 64087
random <- function(n) {  # 0..n-1, our own generator
  seed <<- (seed * 69069 + 1) %% 4294967296
  floor(seed / 4294967296 * n)
}

NON_ASCII <- c("\u00e9", "\u65e5\u672c", "\U0001f44d\U0001f3fd", "e\u0301")

# inputs where ports and originals tend to part ways, of the same type as the recorded value
edges <- function(v) {
  if (!is.atomic(v) || is.null(v)) return(list())
  if (length(v) == 1L) {
    return(switch(typeof(v),
      double = Filter(function(x) !identical(x, v), list(0, 1, -1, NA_real_)),
      integer = Filter(function(x) !identical(x, v), list(0L, 1L, -1L, NA_integer_)),
      logical = list(NA),
      character = list(paste0(substr(v, 1, nchar(v) %/% 2), NON_ASCII[1], substring(v, nchar(v) %/% 2 + 1)),
                       paste0(v, NON_ASCII[2]), paste0(NON_ASCII[3], v), paste0(v, NON_ASCII[4]), "", NA_character_),
      list()))
  }
  out <- list(v[0])
  for (e in edges(v[1])) {
    w <- v
    w[1] <- e
    out[[length(out) + 1L]] <- w
  }
  out
}

# a short version of a recorded value, so edge cases stay cheap when the real inputs are big
small <- function(v) {
  if (is.character(v) && length(v) == 1L && !is.na(v)) return(substr(v, 1, 64))
  if ((is.atomic(v) || is.list(v)) && length(v) > 8L) return(v[1:8])
  v
}

variants <- function(v) {
  if (!(is.atomic(v) || is.list(v)) || length(v) < 2L) return(list())
  out <- list()
  for (n in c(0, 1, 2, 3, 5, 8, 13, 21, 34, 55)) {
    if (n >= length(v)) break
    for (k in seq_len(if (n < 2) 1 else 3)) {
      start <- random(length(v) - n + 1)
      out[[length(out) + 1L]] <- v[seq_len(n) + start]
    }
  }
  out
}

if (mode == "verify") {
  Sys.setenv(FAST_ANYTHING_DISABLE = "1")  # the original R, even if the port is switched on
  env <- new.env(parent = globalenv())
  sys.source(job$module_file, envir = env, keep.source = FALSE)
  original <- get(job$func, envir = env)
  port <- fa$.fa_port(job$lib, original)
  cases <- readRDS(job$cases)
  recorded <- length(cases)

  derived <- list()
  seen <- character()
  start <- proc.time()[["elapsed"]]
  budget <- 1.5  # edge cases first, then shrunk inputs, 1.5 s each
  try_case <- function(args, kind) {
    if (proc.time()[["elapsed"]] - start > budget || length(derived) >= 300) return(invisible())
    key <- tryCatch(fa$.fa_json(unname(args)), error = function(e) NULL)
    if (is.null(key) || key %in% seen) return(invisible())
    seen <<- c(seen, key)
    result <- tryCatch(do.call(original, args), error = function(e) e)
    if (inherits(result, "error") || is.null(tryCatch(fa$.fa_json(result), error = function(e) NULL))) return(invisible())
    derived[[length(derived) + 1L]] <<- list(args = args, result = result, derived = kind)
  }
  plain <- Filter(function(c) is.null(c$raises), cases)
  for (case in head(plain, 20)) {
    base <- lapply(case$args, small)
    for (i in seq_along(base)) {
      for (e in edges(base[[i]])) {
        args <- base
        args[i] <- list(e)
        try_case(args, "edge")
      }
    }
  }
  budget <- proc.time()[["elapsed"]] - start + 1.5
  for (case in plain) {
    for (i in seq_along(case$args)) {
      for (smaller in variants(case$args[[i]])) {
        args <- case$args
        args[i] <- list(smaller)
        try_case(args, "derived")
      }
    }
  }
  cases <- c(cases, derived)

  failures <- list()
  passed <- 0
  for (i in seq_along(cases)) {
    case <- cases[[i]]
    got <- tryCatch(do.call(port, case$args), error = function(e) e)
    failed <- inherits(got, "error")
    if (!is.null(case$raises)) {
      ok <- failed
      expected <- paste("raises", case$raises)
    } else {
      ok <- !failed && fa$.fa_same(case$result, got)
      expected <- fa$.fa_show(case$result)
    }
    if (ok) {
      passed <- passed + 1
      next
    }
    if (length(failures) < 8) {
      shown <- vapply(seq_along(case$args), function(j) {
        n <- names(case$args)[j]
        paste0(if (!is.null(n) && nzchar(n)) paste0(n, " = ") else "", fa$.fa_show(case$args[[j]], 80L))
      }, "")
      label <- switch(if (is.null(case$derived)) "" else case$derived, edge = "(edge) ", derived = "(derived) ", "")
      failures[[length(failures) + 1L]] <- list(
        case = i - 1, call = paste0(label, job$func, "(", paste(shown, collapse = ", "), ")"),
        expected = expected,
        got = if (failed) paste("raised an error:", conditionMessage(got)) else fa$.fa_show(got))
    }
  }

  micro <- NULL
  if (length(cases) && length(failures) == 0 && recorded) {
    best <- function(f) {
      calls <- lapply(head(cases, recorded), `[[`, "args")
      runs <- 0
      spent <- 0
      fastest <- Inf
      while (runs < 3 || (spent < job$budget && runs < 2000)) {
        t <- proc.time()[["elapsed"]]
        for (a in calls) try(do.call(f, a), silent = TRUE)
        took <- proc.time()[["elapsed"]] - t
        fastest <- min(fastest, took)
        spent <- spent + took
        runs <- runs + 1
      }
      per_pass <- if (fastest > 0) fastest else spent / runs  # the timer counts milliseconds
      per_pass / length(calls) * 1e9
    }
    r <- best(original)
    rs <- best(port)
    micro <- list(original_ns = r, rust_ns = rs, speedup = if (rs > 0) r / rs else 0)
  }
  report <- list(total = length(cases), derived = length(derived),
                 edges = sum(vapply(derived, function(c) identical(c$derived, "edge"), TRUE)),
                 passed = passed, failed = length(cases) - passed, failures = failures, micro = micro)
  writeLines(fa$.fa_json(report), job$out, useBytes = TRUE)
}
