"""fast-anything: profile and record inside multiprocessing workers too.

While fast-anything profiles or records an app, it puts the folder with this file first on
PYTHONPATH, so every Python process the app starts loads it. Not meant to be used by hand.
Each multiprocessing worker (fork, spawn or forkserver; Pool and ProcessPoolExecutor included)
gets its own profiler or recorder and leaves its results in FAST_ANYTHING_CHILDREN. The app's own
sitecustomize, if it has one, still runs right after this.
"""

import os
import sys


def _fast_anything_watch_workers():
    import multiprocessing.process

    original = multiprocessing.process.BaseProcess._bootstrap
    if getattr(original, "_fast_anything", False):
        return

    def _bootstrap(self, *args, **kwargs):
        if os.name == "posix":
            # Pool.terminate() (and with Pool(...) as p) ends workers with SIGTERM: exit normally
            # instead, so the worker still leaves its results. Set first, before anything slow.
            import signal

            def _on_term(*_):
                # a second SIGTERM must not cut the worker off while it writes its results
                signal.signal(signal.SIGTERM, signal.SIG_IGN)
                sys.exit(0)

            signal.signal(signal.SIGTERM, _on_term)
        state = None
        try:
            state = sys.modules.get("_fast_anything_state")
            if state is None:  # a fresh interpreter (spawn, forkserver): load the runner
                import importlib.util

                spec = importlib.util.spec_from_file_location("_fast_anything_runner", os.environ["FAST_ANYTHING_RUNNER"])
                runner = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(runner)
                state = runner.child_state()
            state.start()
        except Exception as e:  # never break the app's workers
            sys.stderr.write("fast-anything: can't watch this worker: %r\n" % (e,))
            state = None
        try:
            return original(self, *args, **kwargs)
        finally:
            if state is not None:
                state.finish()

    _bootstrap._fast_anything = True
    multiprocessing.process.BaseProcess._bootstrap = _bootstrap
    if os.name == "nt":
        _fast_anything_graceful_pool_terminate()


def _fast_anything_graceful_pool_terminate():
    """On Windows, Pool.terminate() (and `with Pool(...) as p:`) uses TerminateProcess(), which
    kills workers before BaseProcess._bootstrap's finally block can write their profile/recordings.
    When no tasks are still pending, close the pool and wait briefly for idle workers to exit on
    their own, then hand off to the original terminate()."""
    import multiprocessing.pool
    import threading
    import time

    Pool = multiprocessing.pool.Pool
    original = Pool.terminate
    pool_close = Pool.close
    if getattr(original, "_fast_anything", False):
        return

    grace = 2.0  # total wall-clock cap (seconds) so terminate() never hangs on a stuck worker

    def _terminate(self):
        try:
            pool = getattr(self, "_pool", None)
            workers = list(pool) if pool else []
            cache = getattr(self, "_cache", None)
            state = getattr(self, "_state", None)
            # Only drain when workers are processes (not ThreadPool threads) and _cache is empty.
            # Submitted Pool jobs stay in _cache until their tasks finish, so skipping close()
            # while _cache is non-empty prevents queued work from running after terminate().
            if (
                workers
                and hasattr(workers[0], "terminate")
                and state in (multiprocessing.pool.RUN, multiprocessing.pool.CLOSE)
                and cache is not None
                and not cache
            ):
                if state == multiprocessing.pool.RUN:
                    pool_close(self)
                # Re-check after close() (which rejects new submissions) in case another thread
                # queued work right before close() flipped _state.
                if not cache:
                    deadline = time.monotonic() + grace
                    worker_handler = getattr(self, "_worker_handler", None)
                    if (
                        worker_handler is not None
                        and worker_handler.is_alive()
                        and threading.current_thread() is not worker_handler
                    ):
                        worker_handler.join(max(0.0, deadline - time.monotonic()))
                    for p in list(pool):
                        remaining = deadline - time.monotonic()
                        if remaining <= 0:
                            break
                        if p.is_alive():
                            p.join(remaining)
        except Exception as e:
            if not sys.is_finalizing() and sys.stderr is not None:
                sys.stderr.write("fast-anything: can't drain pool workers: %r\n" % (e,))
        return original(self)

    _terminate._fast_anything = True
    Pool.terminate = _terminate


def _fast_anything_chain():
    """Run the sitecustomize this one hides, if there is one."""
    import importlib.util
    from importlib.machinery import PathFinder

    here = os.path.dirname(os.path.abspath(__file__))
    spec = PathFinder.find_spec("sitecustomize", [p for p in sys.path if os.path.abspath(p or ".") != here])
    if spec is not None and spec.loader is not None:
        spec.loader.exec_module(importlib.util.module_from_spec(spec))


if os.environ.get("FAST_ANYTHING_CHILDREN"):
    _fast_anything_watch_workers()
_fast_anything_chain()
