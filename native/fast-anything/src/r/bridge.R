# fast-anything's R bridge: R values to JSON for a Rust port, the port's reply back to R values,
# and the comparison verify uses. Base R only. Copied into <project>/.fast; not meant to be used
# by hand.

.fa_unsupported <- function(x) {
  stop(structure(class = c("fa_unsupported", "error", "condition"),
                 list(message = class(x)[1], call = NULL)))
}

# JSON string literals for a character vector without NAs
.fa_str <- function(s) {
  s <- enc2utf8(as.character(s))
  s <- gsub("\\", "\\\\", s, fixed = TRUE)
  s <- gsub("\"", "\\\"", s, fixed = TRUE)
  s <- gsub("\n", "\\n", s, fixed = TRUE)
  s <- gsub("\r", "\\r", s, fixed = TRUE)
  s <- gsub("\t", "\\t", s, fixed = TRUE)
  ctrl <- grepl("[\001-\037]", s, useBytes = TRUE)
  if (any(ctrl)) {
    s[ctrl] <- vapply(s[ctrl], function(one) {
      codes <- utf8ToInt(one)
      paste(ifelse(codes < 32, sprintf("\\u%04x", codes), vapply(codes, intToUtf8, "")), collapse = "")
    }, "", USE.NAMES = FALSE)
  }
  paste0("\"", s, "\"")
}

# JSON tokens for a double vector. NA is null; NaN and +-Inf, which JSON has no numbers for, are
# {"$num": ...} (the port reads them with r::Num).
.fa_num <- function(x) {
  out <- sprintf("%.15g", x)
  ok <- is.finite(x)
  inexact <- ok & suppressWarnings(as.numeric(out)) != x
  out[which(inexact)] <- sprintf("%.17g", x[which(inexact)])
  out[is.na(x) & !is.nan(x)] <- "null"
  out[is.nan(x)] <- "{\"$num\":\"NaN\"}"
  out[!is.na(x) & x == Inf] <- "{\"$num\":\"Inf\"}"
  out[!is.na(x) & x == -Inf] <- "{\"$num\":\"-Inf\"}"
  out
}

# An R value as JSON: a length-1 vector is a plain value, a longer one an array, a named one an
# object; lists likewise. Anything with other attributes (factors, data frames, dates...) or of
# another type is unsupported.
.fa_json <- function(x) {
  if (is.null(x)) return("null")
  if (length(setdiff(names(attributes(x)), "names"))) .fa_unsupported(x)
  keys <- names(x)
  if (is.list(x)) {
    parts <- vapply(x, .fa_json, "", USE.NAMES = FALSE)
    if (!is.null(keys)) return(paste0("{", paste0(.fa_str(keys), ":", parts, collapse = ","), "}"))
    return(paste0("[", paste(parts, collapse = ","), "]"))
  }
  parts <- switch(typeof(x),
    double = .fa_num(x),
    integer = ifelse(is.na(x), "null", as.character(x)),
    logical = ifelse(is.na(x), "null", ifelse(x, "true", "false")),
    character = ifelse(is.na(x), "null", .fa_str(ifelse(is.na(x), "", x))),
    .fa_unsupported(x))
  if (!is.null(keys)) return(paste0("{", paste0(.fa_str(keys), ":", parts, collapse = ","), "}"))
  if (length(x) == 1L) return(parts)
  paste0("[", paste(parts, collapse = ","), "]")
}

.fa_load <- function(lib) {
  dll <- dyn.load(lib)
  list(call = getNativeSymbolInfo("fa_r_call", dll), take = getNativeSymbolInfo("fa_r_take", dll))
}

# Call the port with a list of arguments (in the original's parameter order). Its reply comes back
# as R source (c(...), list(...), NA, Inf...), which R's own parser reads.
.fa_invoke <- function(port, args) {
  input <- paste0("[", paste(vapply(args, .fa_json, "", USE.NAMES = FALSE), collapse = ","), "]")
  n <- .C(port$call, input, n = integer(1))$n
  reply <- eval(parse(text = .C(port$take, out = strrep(" ", n))$out, keep.source = FALSE), baseenv())
  if (!is.null(reply$error)) stop(reply$error, call. = FALSE)
  reply$ok
}

# A function with the original's parameters (and defaults) that calls `run` with the argument
# values, in parameter order.
.fa_like <- function(original, run) {
  g <- function() NULL
  formals(g) <- formals(original)
  body(g) <- substitute(RUN(mget(names(formals(sys.function())), envir = environment())), list(RUN = run))
  environment(g) <- environment(original)
  g
}

# The Rust port in place of `original`.
.fa_port <- function(lib, original) {
  port <- .fa_load(lib)
  invoke <- .fa_invoke
  .fa_like(original, function(args) invoke(port, unname(args)))
}

# Same result the way R code sees it: integer and double are both numbers, doubles within 1e-9,
# NA where NA, zero-length vectors and NULL alike, names and list structure must match.
.fa_same <- function(a, b) {
  if (length(a) == 0L && length(b) == 0L && !is.list(a) && !is.list(b)) return(TRUE)
  if (is.list(a) || is.list(b)) {
    if (!(is.list(a) && is.list(b)) || length(a) != length(b) || !identical(names(a), names(b))) return(FALSE)
    return(all(mapply(.fa_same, a, b)))
  }
  if (length(a) != length(b) || !identical(names(a), names(b))) return(FALSE)
  # an NA from Rust has no type; R turns logical NA into any other type where needed
  if ((is.logical(a) && all(is.na(a))) || (is.logical(b) && all(is.na(b)))) {
    return(all(is.na(a)) && all(is.na(b)))
  }
  if (is.numeric(a) && is.numeric(b)) {
    if (!identical(is.na(a), is.na(b)) || !identical(is.nan(a), is.nan(b))) return(FALSE)
    a <- as.double(a[!is.na(a)])
    b <- as.double(b[!is.na(b)])
    inf <- is.infinite(a) | is.infinite(b)
    if (!identical(a[inf], b[inf])) return(FALSE)
    a <- a[!inf]
    b <- b[!inf]
    return(all(a == b | abs(a - b) <= pmax(1e-12, 1e-9 * pmax(abs(a), abs(b)))))
  }
  identical(typeof(a), typeof(b)) && identical(unname(a), unname(b))
}

.fa_show <- function(x, limit = 160L) {
  text <- paste(deparse(x, width.cutoff = 500L, nlines = 3L), collapse = " ")
  if (nchar(text) > limit) paste0(substr(text, 1L, limit - 1L), "\u2026") else text
}
