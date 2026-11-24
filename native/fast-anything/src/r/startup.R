# fast-anything's R startup file: R_PROFILE_USER points here while fast-anything profiles or
# records an app, so the app runs with its own command line. Not meant to be used by hand.
#   FAST_ANYTHING_MODE=profile: Rprof samples the whole run.
#   FAST_ANYTHING_MODE=record:  .fast_anything_wrap(f) records calls to f (fast-anything adds a
#                               line calling it right after the function's definition).
# The user's own startup file (R_PROFILE_USER, ./.Rprofile or ~/.Rprofile) still runs, at the end.

local({
  mode <- Sys.getenv("FAST_ANYTHING_MODE")
  fa <- new.env()
  sys.source(Sys.getenv("FAST_ANYTHING_BRIDGE"), envir = fa)

  if (mode == "profile") {
    out <- Sys.getenv("FAST_ANYTHING_OUT")
    start <- proc.time()[["elapsed"]]
    utils::Rprof(out, interval = 0.005)
    reg.finalizer(fa, function(e) {
      utils::Rprof(NULL)
      writeLines(format(proc.time()[["elapsed"]] - start, digits = 15), paste0(out, ".wall"))
    }, onexit = TRUE)
    # kept reachable, so the finalizer runs when R exits, not at the next garbage collection
    assign(".fast_anything_profile", fa, envir = globalenv())
  }

  if (mode == "record") {
    state <- new.env()
    state$limit <- as.integer(Sys.getenv("FAST_ANYTHING_RECORD_LIMIT", "500"))
    state$calls <- 0
    state$skipped <- 0
    state$unsupported <- character()
    state$kept <- list()
    state$seed <- 64087  # our own generator: the app's random numbers stay as they are

    random <- function(n) {  # 1..n
      state$seed <- (state$seed * 69069 + 1) %% 4294967296
      floor(state$seed / 4294967296 * n) + 1
    }

    record <- function(f, args) {
      state$calls <- state$calls + 1
      slot <- if (length(state$kept) < state$limit) length(state$kept) + 1 else random(state$calls)
      case <- NULL
      if (slot <= state$limit) {
        case <- tryCatch(list(args = args, json = fa$.fa_json(unname(args))), fa_unsupported = function(e) {
          state$skipped <- state$skipped + 1
          state$unsupported <- union(state$unsupported, conditionMessage(e))
          NULL
        })
      }
      result <- tryCatch(do.call(f, args), error = function(e) e)
      if (inherits(result, "error")) {
        if (!is.null(case)) {
          case$raises <- class(result)[1]
          state$kept[[slot]] <- case
        }
        stop(result)
      }
      if (!is.null(case)) {
        json <- tryCatch(fa$.fa_json(result), fa_unsupported = function(e) {
          state$skipped <- state$skipped + 1
          state$unsupported <- union(state$unsupported, conditionMessage(e))
          NULL
        })
        if (!is.null(json)) {
          case$result <- result
          case$result_json <- json
          state$kept[slot] <- list(case)
        }
      }
      result
    }

    assign(".fast_anything_wrap", function(f) fa$.fa_like(f, function(args) record(f, args)), envir = globalenv())

    reg.finalizer(state, function(e) {
      out <- Sys.getenv("FAST_ANYTHING_RECORD_OUT")
      lines <- vapply(state$kept, function(c) {
        tail <- if (is.null(c$raises)) paste0("\"result\":", c$result_json) else paste0("\"raises\":", fa$.fa_str(c$raises))
        paste0("{\"args\":", c$json, ",", tail, "}")
      }, "")
      writeLines(enc2utf8(lines), out, useBytes = TRUE)
      saveRDS(lapply(state$kept, function(c) c[c("args", "result", "raises")]), sub("\\.jsonl$", ".rds", out))
      meta <- list(calls = state$calls, kept = length(state$kept), skipped = state$skipped,
                   unsupported = as.list(state$unsupported), mutates = FALSE)
      writeLines(fa$.fa_json(meta), Sys.getenv("FAST_ANYTHING_RECORD_META"))
    }, onexit = TRUE)
  }

  user <- Sys.getenv("FAST_ANYTHING_USER_RPROFILE")
  if (!nzchar(user)) user <- if (file.exists(".Rprofile")) ".Rprofile" else path.expand("~/.Rprofile")
  if (file.exists(user)) sys.source(user, envir = globalenv())
})
