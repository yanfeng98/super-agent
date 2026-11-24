"""Python side of fast-anything.

The fast-anything CLI copies this file to <project>/.fast/runner.py and runs it as
    python runner.py <mode> <job.json> -- <the app's python arguments>
Modes: profile, record, verify. Not meant to be run by hand.
"""

import ast
import base64
import dataclasses
import functools
import importlib
import importlib.util
import json
import math
import os
import random
import runpy
import sys
import time

DISABLE_ENV = "FAST_ANYTHING_DISABLE"
SKIP_DIRS = {".fast", ".git", "venv", ".venv", "env", ".env", "site-packages", "dist-packages",
             "node_modules", "__pycache__", "build", "dist", ".tox", ".nox"}
# Process state used without a call: `os.environ["X"]` is as much a hidden input as `os.getenv("X")`,
# and `json.load(sys.stdin)` as much I/O as `sys.stdin.read()`. Calls on these are judged by IO_MODULES.
STATE_READS = {"os.environ", "os.environb", "sys.argv", "sys.orig_argv",
               "sys.stdin", "sys.stdout", "sys.stderr", "sys.__stdin__", "sys.__stdout__", "sys.__stderr__"}
IO_NAMES = {"print", "open", "input", "exec", "eval", "compile", "__import__", "breakpoint", "exit", "quit"}
IO_MODULES = {"os", "sys", "subprocess", "socket", "requests", "urllib", "http", "httpx", "aiohttp", "shutil",
              "pathlib", "logging", "sqlite3", "tempfile", "glob", "asyncio", "threading", "multiprocessing", "io"}
NONDET_MODULES = {"random", "time", "datetime", "uuid", "secrets"}
ARRAY_LIBS = {"np", "numpy", "pd", "pandas", "torch", "scipy", "numba", "polars", "pl"}
# Pure callables from the modules above: imported by name, they don't block a function.
PURE_CALLS = {
    "datetime.date", "datetime.datetime", "datetime.time", "datetime.timedelta", "datetime.timezone",
    "datetime.date.fromisoformat", "datetime.date.fromordinal", "datetime.date.fromisocalendar",
    "datetime.datetime.fromisoformat", "datetime.datetime.strptime", "datetime.datetime.combine",
    "datetime.time.fromisoformat", "io.StringIO", "io.BytesIO",
    "pathlib.PurePath", "pathlib.PurePosixPath", "pathlib.PureWindowsPath",
    "os.fspath", "os.fsdecode", "os.fsencode", "os.path.join", "os.path.basename", "os.path.dirname",
    "os.path.split", "os.path.splitext", "os.path.normpath", "os.path.normcase", "os.path.isabs",
    "os.path.commonpath", "os.path.commonprefix", "uuid.UUID", "uuid.uuid3", "uuid.uuid5",
    "http.HTTPStatus", "http.HTTPMethod", "secrets.compare_digest", "glob.escape", "glob.has_magic",
    "socket.inet_aton", "socket.inet_ntoa", "socket.inet_pton", "socket.inet_ntop",
    "socket.htons", "socket.ntohs", "socket.htonl", "socket.ntohl",
}
PURE_MODULES = {"urllib.parse"}


# ------------------------------------------------------------------ values

class Unsupported(Exception):
    """A value that can't be recorded as plain data."""


def record_fields(v):
    """Field names of a dataclass or NamedTuple instance, None for anything else."""
    if isinstance(v, tuple) and isinstance(getattr(type(v), "_fields", None), tuple):
        return list(type(v)._fields)
    if dataclasses.is_dataclass(v) and not isinstance(v, type):
        return [f.name for f in dataclasses.fields(v)]
    return None


# modules recorded classes are looked up in, by the name they were recorded under
MODULES = {}


def find_class(module, qualname):
    target = MODULES.get(module) or importlib.import_module(module)
    for part in qualname.split("."):
        target = getattr(target, part)
    return target


ARRAY_KINDS = "biuf"  # NumPy dtypes recorded: bool, int, unsigned, float
MAX_ARRAY = 1 << 20  # bytes per recorded array, so recordings stay a sane size


def enc_numpy(v):
    """An ndarray or NumPy scalar as plain data (only called for values from the numpy module)."""
    np = sys.modules["numpy"]
    if isinstance(v, np.ndarray):
        if v.dtype.kind not in ARRAY_KINDS:
            raise Unsupported("ndarray of " + str(v.dtype))
        if v.nbytes > MAX_ARRAY:
            raise Unsupported("ndarray over 1 MiB")
        a = np.ascontiguousarray(v)
        return {"$ndarray": [a.dtype.str, list(a.shape), base64.b64encode(a.tobytes()).decode("ascii")]}
    if isinstance(v, np.generic) and v.dtype.kind in ARRAY_KINDS:
        return {"$npscalar": [v.dtype.str, v.item()]}
    raise Unsupported(type(v).__name__)


def enc(v):
    if type(v).__module__ == "numpy" and "numpy" in sys.modules:
        return enc_numpy(v)
    if v is None or isinstance(v, (bool, int, str)):
        return v
    fields = record_fields(v)
    if fields is not None:  # before tuple: a NamedTuple is one
        cls = type(v)
        return {"$obj": [cls.__module__, cls.__qualname__, [[f, enc(getattr(v, f))] for f in fields]]}
    if isinstance(v, float):
        return v if math.isfinite(v) else {"$float": repr(v)}
    if isinstance(v, list):
        return [enc(x) for x in v]
    if isinstance(v, tuple):
        return {"$tuple": [enc(x) for x in v]}
    if isinstance(v, (bytes, bytearray)):
        return {"$bytes": base64.b64encode(bytes(v)).decode("ascii")}
    if isinstance(v, dict):
        return {"$dict": [[enc(k), enc(x)] for k, x in v.items()]}
    if isinstance(v, (set, frozenset)):
        return {"$set": [enc(x) for x in v]}
    raise Unsupported(type(v).__name__)


def dec(v):
    if isinstance(v, list):
        return [dec(x) for x in v]
    if isinstance(v, dict):
        ((tag, x),) = v.items()
        if tag == "$tuple":
            return tuple(dec(i) for i in x)
        if tag == "$bytes":
            return base64.b64decode(x)
        if tag == "$dict":
            return {dec(k): dec(i) for k, i in x}
        if tag == "$set":
            return {dec(i) for i in x}
        if tag == "$float":
            return float(x)
        if tag == "$obj":
            module, qualname, fields = x
            cls = find_class(module, qualname)
            values = {k: dec(i) for k, i in fields}
            try:
                return cls(**values)
            except TypeError:  # fields left out of __init__
                obj = cls.__new__(cls)
                for k, i in values.items():
                    object.__setattr__(obj, k, i)
                return obj
        if tag == "$ndarray":
            import numpy as np
            dtype, shape, data = x
            return np.frombuffer(base64.b64decode(data), dtype=np.dtype(dtype)).reshape(shape).copy()
        if tag == "$npscalar":
            import numpy as np
            return np.dtype(x[0]).type(x[1])
        raise ValueError("unknown tag " + tag)
    return v


def same(a, b):
    """Equality the way a caller would see it: floats within 1e-9, but 3 != 3.0 (it prints
    differently) and a list isn't a tuple (it can't be hashed)."""
    np = sys.modules.get("numpy")
    if np is not None:
        if isinstance(a, np.ndarray) or isinstance(b, np.ndarray):
            if not (isinstance(a, np.ndarray) and isinstance(b, np.ndarray)) or a.shape != b.shape or a.dtype != b.dtype:
                return False
            if a.dtype.kind == "f":
                return bool(np.allclose(a, b, rtol=1e-9, atol=1e-12, equal_nan=True))
            return bool(np.array_equal(a, b))
        # a NumPy scalar compares like the Python number it holds (a.sum() vs a port's f64)
        a = a.item() if isinstance(a, np.generic) else a
        b = b.item() if isinstance(b, np.generic) else b
    if isinstance(a, bool) or isinstance(b, bool):
        return type(a) is type(b) and a == b
    fields = record_fields(a)
    if fields is not None or record_fields(b) is not None:
        return type(a) is type(b) and all(same(getattr(a, f), getattr(b, f)) for f in fields)
    if isinstance(a, float) or isinstance(b, float):
        if not (isinstance(a, float) and isinstance(b, float)):
            return False
        if math.isnan(a) or math.isnan(b):
            return math.isnan(a) and math.isnan(b)
        return math.isclose(a, b, rel_tol=1e-9, abs_tol=1e-12)
    if isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)):
        if isinstance(a, tuple) != isinstance(b, tuple):
            return False
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    return a == b


def show(v, limit=160):
    text = repr(v)
    return text if len(text) <= limit else text[: limit - 1] + "…"


# ------------------------------------------------------------------ running the app

def parse_target(argv):
    args = list(argv)
    while args and args[0].startswith("-") and args[0] != "-m":
        flag = args.pop(0)
        if flag == "-c":
            sys.exit("fast-anything: 'python -c' isn't supported; point it at a script or a module")
        if flag in ("-X", "-W") and args:
            args.pop(0)
    if not args:
        sys.exit("fast-anything: give the app as 'python script.py ...' or 'python -m package ...'")
    if args[0] == "-m":
        if len(args) < 2:
            sys.exit("fast-anything: -m needs a module name")
        return "module", args[1], args[2:]
    return "script", args[0], args[1:]


def prepare_path(kind, target):
    """Make sys.path look like a normal 'python ...' run. Returns (entry file, import base)."""
    here = os.path.dirname(os.path.abspath(__file__))
    sys.path[:] = [p for p in sys.path if os.path.abspath(p or ".") != here]
    if kind == "script":
        entry = os.path.abspath(target)
        base = os.path.dirname(entry)
        sys.path.insert(0, base)
        return entry, base
    base = os.getcwd()
    sys.path.insert(0, base)
    entry = None
    try:
        spec = importlib.util.find_spec(target)
        if spec is not None and spec.submodule_search_locations:
            spec = importlib.util.find_spec(target + ".__main__")
        if spec is not None and spec.origin:
            entry = os.path.abspath(spec.origin)
    except (ImportError, ValueError):
        pass
    return entry, base


def run_target(kind, target, rest):
    sys.argv = [target] + list(rest)
    try:
        if kind == "script":
            runpy.run_path(target, run_name="__main__")
        else:
            runpy.run_module(target, run_name="__main__", alter_sys=True)
    except SystemExit as e:
        if e.code not in (None, 0):
            raise


# ------------------------------------------------------------------ static analysis

_files = {}


def parse_file(path):
    if path not in _files:
        try:
            with open(path, encoding="utf-8") as f:
                src = f.read()
            tree = ast.parse(src)
        except (OSError, SyntaxError, UnicodeDecodeError, ValueError):
            _files[path] = None
            return None
        by_line, top = {}, set()

        def walk(node, prefix, kind):
            for child in ast.iter_child_nodes(node):
                if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    qual = prefix + child.name
                    for line in [child.lineno] + [d.lineno for d in child.decorator_list]:
                        by_line[line] = (child, qual, kind)
                    if kind == "function":
                        top.add(child.name)
                    walk(child, qual + ".<locals>.", "nested")
                elif isinstance(child, ast.ClassDef):
                    walk(child, prefix + child.name + ".", "method" if kind == "function" else "nested")
                else:
                    walk(child, prefix, kind)

        walk(tree, "", "function")
        guard = next((n.lineno for n in tree.body if isinstance(n, ast.If) and is_main_guard(n.test)), None)
        records = {n.name: record_class_fields(n) for n in tree.body
                   if isinstance(n, ast.ClassDef) and is_record_class(n)}
        _files[path] = (src, by_line, top, guard, records, module_names(tree))
    return _files[path]


# --- names that stand for an imported module or function, by scope

# Nodes with a scope of their own: names bound inside them don't reach the code around them.
SCOPES = (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda, ast.ClassDef,
          ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)
BLOCKS = ("body", "orelse", "finalbody", "handlers", "cases")


def imports_in(node):
    """name -> dotted names, for an import statement. `import subprocess as sp` gives sp: subprocess,
    `from os import system` gives system: os.system; a relative import gives () (the app's own)."""
    out = {}
    if isinstance(node, ast.Import):
        for a in node.names:
            name = a.asname or a.name.split(".")[0]
            out.setdefault(name, set()).add(tuple(a.name.split(".")) if a.asname else (name,))
    elif isinstance(node, ast.ImportFrom) and node.module != "__future__":
        for a in node.names:
            if a.name != "*":
                target = tuple(node.module.split(".")) + (a.name,) if node.module and node.level == 0 else ()
                out.setdefault(a.asname or a.name, set()).add(target)
    return out


def binds(node):
    """Names `node` binds in the scope it sits in, other than by importing."""
    if isinstance(node, ast.Name) and isinstance(node.ctx, (ast.Store, ast.Del)):
        return [node.id]
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
        return [node.name]
    if isinstance(node, (ast.ExceptHandler, ast.MatchAs, ast.MatchStar)) and node.name:
        return [node.name]
    if isinstance(node, ast.MatchMapping) and node.rest:
        return [node.rest]
    return []


def evaluated_outside(scope):
    """The parts of a scope node that run in the scope around it: decorators, defaults, class bases,
    a comprehension's first iterable."""
    if isinstance(scope, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
        args = scope.args
        parts = list(args.defaults) + [d for d in args.kw_defaults if d is not None]
        return parts + list(getattr(scope, "decorator_list", []))
    if isinstance(scope, ast.ClassDef):
        return list(scope.decorator_list) + list(scope.bases) + [k.value for k in scope.keywords]
    return [scope.generators[0].iter]


def header_binds(st):
    """What a statement binds outside its blocks (assignment, for and with targets...), not entering scopes."""
    if isinstance(st, SCOPES):
        return binds(st)
    if isinstance(st, ast.AnnAssign) and st.value is None:
        return []  # `x: int` declares, it doesn't bind
    names, stack = binds(st), [v for f, v in ast.iter_fields(st) if f not in BLOCKS]
    while stack:
        v = stack.pop()
        if isinstance(v, list):
            stack.extend(v)
        elif isinstance(v, ast.AST) and not isinstance(v, SCOPES):
            names += binds(v)
            stack.extend(ast.iter_child_nodes(v))
    return names


def bound_at(st):
    """Where a statement's bindings take effect: once its header ran (an assignment's value, a with's
    context, a for's iterable, a def's defaults), so `sp = sp.run()` still calls the imported `sp`."""
    ends = [(n.end_lineno, n.end_col_offset) for f, v in ast.iter_fields(st) if f not in BLOCKS
            for top in (v if isinstance(v, list) else [v]) if isinstance(top, ast.AST)
            for n in ast.walk(top) if getattr(n, "end_lineno", None)]  # a withitem has no position of its own
    return max(ends) if ends else (getattr(st, "lineno", 0), getattr(st, "col_offset", 0))


def bindings(stmts, maybe=False, out=None):
    """(position, name, dotted names, maybe) for each binding in `stmts`, in order. An import binds what it
    imports, anything else binds nothing known. A binding inside an if/try/with/for/while/match block, or a
    for target, may or may not have happened."""
    out = [] if out is None else out
    for st in stmts:
        pos = bound_at(st)
        for name, targets in imports_in(st).items():
            out.append((pos, name, frozenset(targets), maybe))
        if not isinstance(st, (ast.Import, ast.ImportFrom)):
            loop = isinstance(st, (ast.For, ast.AsyncFor))
            out += [(pos, name, frozenset(), maybe or loop) for name in header_binds(st)]
        if not isinstance(st, SCOPES):
            for f in BLOCKS:
                bindings(getattr(st, f, None) or [], True, out)
    return out


def state(entries, names=None, before=None):
    """What each name stands for after `entries` (up to position `before`): a later plain binding replaces
    an earlier one, a maybe-binding adds to it."""
    names = dict(names or {})
    for pos, name, targets, maybe in entries:
        if before is None or pos < before:
            names[name] = names.get(name, frozenset()) | targets if maybe else targets
    return names


def module_names(tree):
    """What each name stands for at module level, after the whole module ran (imported names only)."""
    return {name: targets for name, targets in state(bindings(tree.body)).items() if targets}


def scope_bindings(scope):
    """The bindings of a scope node's own body, its parameters first."""
    if isinstance(scope, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
        a = scope.args
        params = [((0, 0), x.arg, frozenset(), False)
                  for x in a.posonlyargs + a.args + a.kwonlyargs + [v for v in (a.vararg, a.kwarg) if v]]
        body = scope.body if isinstance(scope.body, list) else []
        return params + bindings(body)
    if isinstance(scope, ast.ClassDef):
        return bindings(scope.body)
    # a comprehension binds its targets
    return [((0, 0), name, frozenset(), False) for g in scope.generators for n in ast.walk(g.target) for name in binds(n)]


def resolved_calls(function, module_level):
    """Calls in `function` (nested scopes included) whose first name stands for an imported module or
    function at that call, with what it stands for. Follows Python's scoping: a scope's own bindings hide
    the outer ones (the last one before the call counts), a class body is no scope for the functions in
    it, and decorators, defaults and a comprehension's first iterable run in the scope around."""
    found = []
    flat = {}

    def lookup(env, name, at):
        local, outer = env
        if name in local:
            entries, imported = local[name]
            if not imported:
                return frozenset()  # a plain local
            return state(entries, before=at).get(name, frozenset())
        return outer.get(name, frozenset())

    def visit(node, env, fn_env, skip):
        for child in ast.iter_child_nodes(node):
            if id(child) in skip:
                continue
            if isinstance(child, SCOPES):
                enter(child, env, fn_env)
                continue
            if isinstance(child, ast.Call):
                target = dotted(child.func)
                if target:
                    origins = lookup(env, target[0], (child.lineno, child.col_offset))
                    if origins:
                        found.append((target, origins))
            visit(child, env, fn_env, skip)

    def flatten(env):
        """An env as plain names, for a nested scope to start from (its own bindings come on top)."""
        if id(env) not in flat:
            local, outer = env
            names = dict(outer)
            for name, (entries, _) in local.items():
                names[name] = state(entries).get(name, frozenset())
            flat[id(env)] = (env, names)  # keep env alive so its id isn't reused
        return flat[id(env)][1]

    def enter(scope, env, fn_env):
        heads = evaluated_outside(scope)
        for h in heads:
            visit(ast.Expr(value=h), env, fn_env, frozenset())
        entries = {}
        for entry in scope_bindings(scope):
            entries.setdefault(entry[1], []).append(entry)
        own = {name: (es, any(e[2] for e in es)) for name, es in entries.items()}
        is_class = isinstance(scope, ast.ClassDef)
        inner = (own, flatten(fn_env))  # a class body sees the enclosing function, never an outer class body
        visit(scope, inner, fn_env if is_class else inner, frozenset(map(id, heads)))

    enter(function, ({}, module_level), ({}, module_level))
    return found


def resolved_effect(full):
    """("io" | "nondet", name) for a call resolved through an import, or None."""
    name = ".".join(full)
    if name in PURE_CALLS or any(name.startswith(m + ".") for m in PURE_MODULES):
        return None
    if len(full) < 2:
        return None  # a module isn't callable: the name was rebound to something else
    head = full[0]
    if head == "builtins" and len(full) == 2 and full[1] in IO_NAMES:
        return "io", full[1]
    if head in IO_MODULES:
        return "io", name
    if head in NONDET_MODULES or (head in ARRAY_LIBS and len(full) > 2 and full[1] == "random"):
        return "nondet", name
    return None


def is_record_class(node):
    """@dataclass (or @dataclasses.dataclass(...)) or a NamedTuple subclass."""
    for d in node.decorator_list:
        name = dotted(d.func if isinstance(d, ast.Call) else d)
        if name and name[-1] == "dataclass":
            return True
    return any((dotted(b) or [""])[-1] == "NamedTuple" for b in node.bases)


def record_class_fields(node):
    """(name, annotation) of each field, in order; ClassVars aren't fields."""
    return [(n.target.id, ast.unparse(n.annotation)) for n in node.body
            if isinstance(n, ast.AnnAssign) and isinstance(n.target, ast.Name)
            and "ClassVar" not in ast.unparse(n.annotation)]


def is_main_guard(test):
    """if __name__ == "__main__" (either way round)."""
    if not (isinstance(test, ast.Compare) and len(test.ops) == 1 and isinstance(test.ops[0], ast.Eq)):
        return False
    sides = [test.left, test.comparators[0]]
    names = [s for s in sides if isinstance(s, ast.Name) and s.id == "__name__"]
    consts = [s for s in sides if isinstance(s, ast.Constant) and s.value == "__main__"]
    return len(names) == 1 and len(consts) == 1


def dotted(node):
    parts = []
    while isinstance(node, ast.Attribute):
        parts.append(node.attr)
        node = node.value
    if isinstance(node, ast.Name):
        parts.append(node.id)
        return list(reversed(parts))
    return None


def module_name(path, entry, base):
    ap = os.path.abspath(path)
    if entry and os.path.normcase(ap) == os.path.normcase(entry):
        return "__main__"
    try:
        rel = os.path.relpath(ap, base)
    except ValueError:
        return None
    if rel.startswith(os.pardir) or not rel.endswith(".py"):
        return None
    parts = rel[:-3].split(os.sep)
    if parts[-1] == "__init__":
        parts.pop()
    if not parts or not all(p.isidentifier() for p in parts):
        return None
    return ".".join(parts)


def describe(path, line, name, entry, base):
    parsed = parse_file(path)
    if parsed is None:
        return None
    src, by_line, top, guard, records, module_level_names = parsed
    hit = by_line.get(line)
    if hit is None or hit[0].name != name:
        return None
    node, qualname, kind = hit
    module = module_name(path, entry, base)

    blockers, warnings, factor = [], [], 1.0
    if module is None:
        blockers.append("can't be imported from the app's folder")
    elif module == "__main__" and guard is None:
        blockers.append("lives in the entry script, which runs on import: put that code under if __name__ == '__main__'")
    elif module == "__main__" and node.lineno > guard:
        blockers.append("defined inside the if __name__ == '__main__' block")
    owner = None
    if kind == "method" and any(isinstance(d, ast.Name) and d.id == "staticmethod" for d in node.decorator_list):
        owner = qualname.rsplit(".", 1)[0]
    elif kind == "method":
        blockers.append("method (module-level functions and @staticmethods only for now)")
    elif kind == "nested":
        blockers.append("nested function")
    if isinstance(node, ast.AsyncFunctionDef):
        blockers.append("async")

    args = node.args
    if args.vararg or args.kwarg:
        warnings.append("takes *args/**kwargs")
        factor *= 0.7

    io, nondet, arrays, helpers = set(), set(), set(), set()
    called = set()  # the a.b of a call a.b.c(...), which the call check already judged
    for sub in ast.walk(node):
        if isinstance(sub, ast.Call):
            n = sub.func
            while isinstance(n, ast.Attribute):
                called.add(id(n))
                n = n.value
    for sub in ast.walk(node):
        if isinstance(sub, ast.Attribute) and id(sub) not in called:
            read = ".".join(dotted(sub) or [])
            if read in STATE_READS:
                io.add(read)
    is_generator = writes_globals = False
    for sub in ast.walk(node):
        if isinstance(sub, (ast.Yield, ast.YieldFrom)):
            is_generator = True
        elif isinstance(sub, (ast.Global, ast.Nonlocal)):
            writes_globals = True
        elif isinstance(sub, ast.Call):
            target = dotted(sub.func)
            if not target:
                continue
            head = target[0]
            if len(target) == 1:
                if head in IO_NAMES:
                    io.add(head)
                elif head in top and head != node.name:
                    helpers.add(head)
            elif head in IO_MODULES:
                io.add(".".join(target))
            elif head in NONDET_MODULES:
                nondet.add(".".join(target))
            elif head in ARRAY_LIBS:
                arrays.add(head)
    # the same calls through an import: `sp.run` is subprocess.run, `system` is os.system
    for target, origins in resolved_calls(node, module_level_names):
        head = target[0]
        if (head in IO_NAMES if len(target) == 1 else head in IO_MODULES or head in NONDET_MODULES):
            continue  # already caught by name
        for origin in origins:
            effect = origin and resolved_effect(list(origin) + target[1:])
            if effect:
                (io if effect[0] == "io" else nondet).add(effect[1])
    if is_generator:
        blockers.append("generator (yield)")
    if writes_globals:
        blockers.append("writes global state")
    if io:
        blockers.append("does I/O: " + ", ".join(sorted(io)[:3]))
    if nondet:
        blockers.append("not deterministic: " + ", ".join(sorted(nondet)[:3]))
    if arrays:
        warnings.append("already uses " + ", ".join(sorted(arrays)) + ", less to gain")
        factor *= 0.4
    if helpers:
        warnings.append("calls " + ", ".join(sorted(helpers)[:3]) + ", they get ported together with it")

    positional = args.posonlyargs + args.args
    defaults = [None] * (len(positional) - len(args.defaults)) + list(args.defaults)
    params = [{"name": a.arg, "annotation": ast.unparse(a.annotation) if a.annotation else None,
               "default": ast.unparse(d) if d is not None else None}
              for a, d in zip(positional, defaults)]
    params += [{"name": a.arg, "annotation": ast.unparse(a.annotation) if a.annotation else None,
                "default": ast.unparse(d) if d is not None else None, "kwonly": True}
               for a, d in zip(args.kwonlyargs, args.kw_defaults)]
    returns = ast.unparse(node.returns) if node.returns else None
    if returns is None or any(p["annotation"] is None for p in params):
        warnings.append("missing type hints, types are inferred from recorded calls")
        factor *= 0.9

    # dataclasses and NamedTuples from this file that the signature uses, with their fields
    names = {n.id for p in [node.returns] + [a.annotation for a in positional + args.kwonlyargs] if p is not None
             for n in ast.walk(p) if isinstance(n, ast.Name)}
    used = {name: [{"name": f, "annotation": a} for f, a in fields] for name, fields in records.items() if name in names}

    lines = src.splitlines()
    first = min([node.lineno] + [d.lineno for d in node.decorator_list])
    return {
        "records": used,
        "qualname": qualname,
        "module": module,
        "kind": kind,
        "owner": owner,
        "end_line": node.end_lineno,
        "params": params,
        "returns": returns,
        "blockers": blockers,
        "warnings": warnings,
        "portability": 0.0 if blockers else factor,
        "calls": sorted(helpers),
        "source": "\n".join(lines[first - 1: node.end_lineno]),
    }


def project_relative(root, path):
    try:
        rel = os.path.relpath(os.path.abspath(path), root)
    except ValueError:
        return None
    if rel == os.pardir or rel.startswith(os.pardir + os.sep):
        return None
    if any(part in SKIP_DIRS for part in rel.split(os.sep)[:-1]):
        return None
    return rel


# ------------------------------------------------------------------ modes

def cmd_profile(job, argv):
    import cProfile
    import pstats

    kind, target, rest = parse_target(argv)
    entry, base = prepare_path(kind, target)
    root = job["root"]
    prof = cProfile.Profile()
    sys.modules["_fast_anything_state"] = ChildState("profile", os.environ.get("FAST_ANYTHING_CHILDREN", ""))
    start = time.perf_counter()
    prof.enable()
    try:
        run_target(kind, target, rest)
    finally:
        prof.disable()
    wall = time.perf_counter() - start
    del sys.modules["_fast_anything_state"]

    stats = pstats.Stats(prof)
    for worker in children(".prof"):  # what the app's multiprocessing workers ran
        try:
            stats.add(worker)
        except (EOFError, ValueError, TypeError, OSError):
            pass  # a worker that died before it finished writing
    rows = []
    for (path, line, name), (_cc, ncalls, tottime, cumtime, _callers) in stats.stats.items():
        if name.startswith("<") or not path or path.startswith("<"):
            continue
        rel = project_relative(root, path)
        if rel is not None:
            rows.append((path, rel, line, name, ncalls, tottime, cumtime))
    rows.sort(key=lambda r: r[6], reverse=True)

    functions = []
    for path, rel, line, name, ncalls, tottime, cumtime in rows[: job.get("max", 40)]:
        info = describe(path, line, name, entry, base)
        if info is None:
            continue
        info.update(file=rel.replace(os.sep, "/"), line=line, name=name,
                    ncalls=ncalls, tottime=tottime, cumtime=cumtime)
        functions.append(info)

    try:
        base_rel = os.path.relpath(base, root)
    except ValueError:
        base_rel = base
    with open(job["out"], "w", encoding="utf-8") as f:
        json.dump({"wall": wall, "base": base_rel, "functions": functions}, f)


class Recorder:
    """Keeps a uniform sample of up to `limit` calls to the wrapped function (reservoir sampling)."""

    def __init__(self, limit, seed):
        self.limit, self.rng = limit, random.Random(seed)
        self.reset()

    def reset(self):
        self.kept, self.calls, self.skipped, self.unsupported, self.mutates = [], 0, 0, set(), False

    def store(self, slot, case):
        if len(self.kept) < self.limit:
            self.kept.append(case)
        else:
            self.kept[slot % self.limit] = case

    def wrap(self, original):
        @functools.wraps(original)
        def recorder(*args, **kwargs):
            self.calls += 1
            slot = len(self.kept) if len(self.kept) < self.limit else self.rng.randrange(self.calls)
            case = None
            if slot < self.limit:
                try:  # encode before the call, in case the function mutates its arguments
                    case = {"args": enc(list(args)), "kwargs": {k: enc(v) for k, v in kwargs.items()}}
                except Unsupported as e:
                    self.skipped += 1
                    self.unsupported.add(str(e))
            try:
                result = original(*args, **kwargs)
            except Exception as e:
                if case is not None:
                    case["raises"] = type(e).__name__
                    self.store(slot, case)
                raise
            if case is not None:
                try:
                    if enc(list(args)) != case["args"] or {k: enc(v) for k, v in kwargs.items()} != case["kwargs"]:
                        self.mutates = True
                except Unsupported:
                    pass
                try:
                    case["result"] = enc(result)
                    self.store(slot, case)
                except Unsupported as e:
                    self.skipped += 1
                    self.unsupported.add(str(e))
            return result

        return recorder

    def install(self, job, main_module=None):
        """Wrap the function where the app looks it up."""
        if job["module"] == "__main__":
            # the entry script wraps the function itself, via a block fast-anything inserted
            import builtins

            builtins.__fast_anything_wrap__ = self.wrap
            if main_module is None:
                return
            module = main_module  # a spawned worker: the entry script is already imported
        else:
            module = importlib.import_module(job["module"])
        owner = getattr(module, job["owner"]) if job.get("owner") else module
        original = getattr(owner, job["func"])
        if not hasattr(original, "__wrapped__"):
            wrapped = self.wrap(original)
            setattr(owner, job["func"], staticmethod(wrapped) if job.get("owner") else wrapped)

    def write(self, out, meta):
        with open(out, "w", encoding="utf-8") as f:
            for case in self.kept:
                f.write(json.dumps(case, ensure_ascii=False) + "\n")
        with open(meta, "w", encoding="utf-8") as f:
            json.dump({"calls": self.calls, "kept": len(self.kept), "skipped": self.skipped,
                       "unsupported": sorted(self.unsupported), "mutates": self.mutates}, f)

    def absorb(self, out, meta):
        """Add what a worker recorded, keeping at most `limit` calls, sampled across all processes."""
        with open(meta, encoding="utf-8") as f:
            m = json.load(f)
        with open(out, encoding="utf-8") as f:
            cases = [json.loads(line) for line in f if line.strip()]
        self.calls += m["calls"]
        self.skipped += m["skipped"]
        self.unsupported.update(m["unsupported"])
        self.mutates = self.mutates or m["mutates"]
        self.kept += cases
        if len(self.kept) > self.limit:
            self.kept = self.rng.sample(self.kept, self.limit)


class ChildState:
    """What a multiprocessing worker does while fast-anything profiles or records the app: its own
    profiler or recorder, written to `out_dir` when the worker ends."""

    def __init__(self, mode, out_dir, recorder=None):
        self.mode, self.out_dir, self.recorder, self.profiler = mode, out_dir, recorder, None

    def start(self):
        if self.mode == "profile":
            import cProfile

            sys.setprofile(None)  # a forked worker inherits the parent's profiler
            self.profiler = cProfile.Profile()
            self.profiler.enable()
        elif self.recorder is not None:
            self.recorder.reset()  # a forked worker inherits the parent's calls so far

    def finish(self):
        if os.name == "posix":
            import signal

            signal.signal(signal.SIGTERM, signal.SIG_IGN)  # don't stop halfway through writing
        base = os.path.join(self.out_dir, str(os.getpid()))
        # written under a temporary name, then renamed, so the parent never reads half a file
        if self.profiler is not None:
            self.profiler.disable()
            self.profiler.dump_stats(base + ".tmp")
            os.replace(base + ".tmp", base + ".prof")
        elif self.recorder is not None:
            self.recorder.write(base + ".tmp", base + ".meta.tmp")
            os.replace(base + ".tmp", base + ".jsonl")
            os.replace(base + ".meta.tmp", base + ".meta.json")


def child_state():
    """The ChildState for a worker started as a fresh interpreter (spawn, forkserver)."""
    with open(os.environ["FAST_ANYTHING_JOB"], encoding="utf-8") as f:
        job = json.load(f)
    mode, out_dir = os.environ["FAST_ANYTHING_MODE"], os.environ["FAST_ANYTHING_CHILDREN"]
    recorder = None
    if mode == "record":
        os.environ[DISABLE_ENV] = "1"
        recorder = Recorder(int(job["limit"]), os.getpid())
        recorder.install(job, sys.modules.get("__main__"))
    state = ChildState(mode, out_dir, recorder)
    sys.modules["_fast_anything_state"] = state
    return state


def children(suffix):
    """Files the app's multiprocessing workers left (see ChildState)."""
    out_dir = os.environ.get("FAST_ANYTHING_CHILDREN")
    if not out_dir or not os.path.isdir(out_dir):
        return []
    return sorted(os.path.join(out_dir, n) for n in os.listdir(out_dir) if n.endswith(suffix))


def cmd_record(job, argv):
    kind, target, rest = parse_target(argv)
    prepare_path(kind, target)
    os.environ[DISABLE_ENV] = "1"  # always record the original Python
    recorder = Recorder(int(job["limit"]), 0xFA57)
    recorder.install(job)
    sys.modules["_fast_anything_state"] = ChildState("record", os.environ.get("FAST_ANYTHING_CHILDREN", ""), recorder)
    run_target(kind, target, rest)
    del sys.modules["_fast_anything_state"]
    for out in children(".jsonl"):
        try:
            recorder.absorb(out, out[: -len(".jsonl")] + ".meta.json")
        except (OSError, ValueError, KeyError):
            pass  # a worker that died before it finished writing
    recorder.write(job["out"], job["meta"])


def cmd_verify(job, _argv):
    os.environ[DISABLE_ENV] = "1"
    sys.path.insert(0, job["base"])
    sys.path.insert(0, job["libdir"])
    fast = getattr(importlib.import_module(job["ext"]), job["func"])
    if job["module"] == "__main__":
        # import the entry script under another name: its if __name__ == "__main__" block won't run
        spec = importlib.util.spec_from_file_location("_fast_anything_entry", job["module_file"])
        entry = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = entry
        spec.loader.exec_module(entry)
        module = entry
        # as in the app, the entry script is __main__: for classes recorded there, and for ports
        # that look up a class with py.import("__main__")
        MODULES["__main__"] = entry
        sys.modules["__main__"] = entry
    else:
        module = importlib.import_module(job["module"])
    original = getattr(getattr(module, job["owner"]) if job.get("owner") else module, job["func"])

    with open(job["cases"], encoding="utf-8") as f:
        cases = [json.loads(line) for line in f if line.strip()]
    recorded = len(cases)
    derived = derive(original, cases, job.get("derive_budget", 3.0), job.get("derive_limit", 200))
    cases += derived

    report = {"total": len(cases), "derived": len(derived), "passed": 0, "failed": 0, "failures": [],
              "edges": sum(1 for c in derived if c.get("derived") == "edge")}
    calls = []
    for i, case in enumerate(cases):
        args = dec(case["args"])
        kwargs = {k: dec(v) for k, v in case.get("kwargs", {}).items()}
        calls.append((args, kwargs))
        try:
            got, err = fast(*args, **kwargs), None
        except (KeyboardInterrupt, SystemExit):
            raise
        except BaseException as e:  # Rust panics arrive as PanicException (a BaseException)
            got, err = None, e
        if "raises" in case:
            ok, expected = raises_like(err, case["raises"]), "raises " + case["raises"]
        else:
            want = dec(case["result"])
            ok, expected = err is None and same(want, got), show(want)
        if ok:
            report["passed"] += 1
            continue
        report["failed"] += 1
        if len(report["failures"]) < 8:
            shown = [show(a, 80) for a in args] + [k + "=" + show(v, 80) for k, v in kwargs.items()]
            report["failures"].append({
                "case": i,
                "call": {"edge": "(edge) ", True: "(derived) "}.get(case.get("derived"), "") + job["func"] + "(" + ", ".join(shown) + ")",
                "expected": expected,
                "got": show(got) if err is None else "raised " + type(err).__name__ + ": " + show(str(err), 200),
            })

    if cases and report["failed"] == 0:
        calls = calls[:recorded]
        py = best_pass(original, calls, job.get("budget", 0.5))
        rs = best_pass(fast, calls, job.get("budget", 0.5))
        report["micro"] = {"original_ns": py / len(calls) * 1e9, "rust_ns": rs / len(calls) * 1e9,
                           "speedup": py / rs if rs > 0 else 0.0}

    with open(job["out"], "w", encoding="utf-8") as f:
        json.dump(report, f)


SLICE_LENGTHS = (0, 1, 2, 3, 5, 8, 13, 21, 34, 55)


def is_array(value):
    np = sys.modules.get("numpy")
    return np is not None and isinstance(value, np.ndarray) and value.ndim >= 1


def variants(value, rng):
    """Smaller versions of a list/str/bytes/tuple argument (or an array, along its first axis): empty,
    tiny, and random slices."""
    if not (isinstance(value, (list, tuple, str, bytes)) or is_array(value)) or len(value) < 2 or record_fields(value) is not None:
        return []
    n = len(value)
    out = []
    for length in SLICE_LENGTHS:
        if length < n:
            for _ in range(1 if length < 2 else 3):
                start = rng.randrange(n - length + 1)
                out.append(value[start:start + length])
    return out


NON_ASCII = ("\u00e9", "\u65e5\u672c", "\U0001f44d\U0001f3fd", "e\u0301")
BIG = 4096  # the longest "huge" input, in items or characters
EDGE_SAMPLE, EDGE_LIMIT = 20, 100
GROW = (4, 16, 64, 256, 512)  # how much bigger, step by step, than the short version of an input


def edges(value):
    """Inputs where ports and originals tend to part ways, of the same type as the recorded value."""
    if isinstance(value, bool) or record_fields(value) is not None:
        return []
    if isinstance(value, int):
        return [x for x in (0, 1, -1) if x != value]
    if isinstance(value, float):
        return [math.nan, math.inf, -math.inf, -0.0, 0.0, 1e308, 5e-324]
    if isinstance(value, str):
        mid = len(value) // 2
        return [value[:mid] + NON_ASCII[0] + value[mid:], value + NON_ASCII[1], NON_ASCII[2] + value,
                value + NON_ASCII[3], ""]
    if isinstance(value, bytes):
        return [b"", value + b"\x00", b"\xff" + value]
    if is_array(value):
        if value.ndim != 1 or value.dtype.kind not in "iuf" or not len(value):
            return []
        out = [value[:0].copy()]
        for x in {"f": (math.nan, math.inf, -math.inf, -0.0), "i": (0, -1), "u": (0,)}[value.dtype.kind]:
            changed = value.copy()
            changed[0] = x
            out.append(changed)
        return out
    if isinstance(value, (list, tuple)) and value:
        out = [type(value)()]
        for e in edges(value[0])[:4]:
            out.append((e,) + tuple(value[1:]) if isinstance(value, tuple) else [e] + list(value[1:]))
        return out
    return []


def small(value):
    """A short version of a recorded value, so edge cases stay cheap when the real inputs are big."""
    if record_fields(value) is not None:
        return value
    if isinstance(value, (list, tuple)) or is_array(value):
        return value[:8]
    if isinstance(value, (str, bytes)):
        return value[:64]
    return value


def bigger(value, factor):
    """The value repeated factor times (None past BIG items or characters)."""
    if is_array(value):
        if not len(value) or len(value) * factor > BIG:
            return None
        return sys.modules["numpy"].concatenate([value] * factor)
    if not isinstance(value, (list, tuple, str, bytes)) or not value or len(value) * factor > BIG:
        return None
    if record_fields(value) is not None:
        return None
    return value * factor


def derive_edges(original, cases, budget, limit):
    """Edge cases built from a sample of recorded calls: special numbers, non-ASCII text, empty and
    much bigger inputs. The original answers them; inputs it rejects are skipped."""
    import copy

    derived, seen, start = [], set(), time.perf_counter()

    def left():
        return budget - (time.perf_counter() - start)

    def attempt(args, kwargs):
        try:
            encoded = {"args": enc(args), "kwargs": {k: enc(v) for k, v in kwargs.items()}}
            key = json.dumps(encoded, sort_keys=True)
            if key in seen:
                return None
            seen.add(key)
            t = time.perf_counter()
            result = original(*copy.deepcopy(args), **copy.deepcopy(kwargs))
            took = time.perf_counter() - t
            encoded["result"] = enc(result)
        except Exception:
            return None  # outside the function's domain: not a fair test
        encoded["derived"] = "edge"
        derived.append(encoded)
        return took

    grown = set()
    for case in [c for c in cases if "raises" not in c][:EDGE_SAMPLE]:
        args = dec(case["args"])
        kwargs = {k: dec(v) for k, v in case.get("kwargs", {}).items()}
        base = [small(a) for a in args]
        for i, arg in enumerate(args):
            for e in edges(base[i]):
                if len(derived) >= limit or left() <= 0:
                    return derived
                attempt(base[:i] + [e] + base[i + 1:], kwargs)
            if i in grown or bigger(base[i], 4) is None:
                continue
            # much bigger inputs, only while their time (assumed quadratic) fits the budget
            grown.add(i)
            try:
                t = time.perf_counter()
                original(*copy.deepcopy(base), **copy.deepcopy(kwargs))
                took = time.perf_counter() - t
            except Exception:
                continue
            for factor in GROW:
                big = bigger(base[i], factor)
                if big is None or len(derived) >= limit or took * 16 > left() / 2:
                    break
                took = attempt(base[:i] + [big] + base[i + 1:], kwargs)
                if took is None:
                    break
        kw_base = {k: small(v) for k, v in kwargs.items()}
        for k, v in kwargs.items():
            for e in edges(kw_base[k]):
                if len(derived) >= limit or left() <= 0:
                    return derived
                next_kw = dict(kwargs)
                next_kw[k] = e
                attempt(base, next_kw)
            if ("kw", k) in grown or bigger(kw_base[k], 4) is None:
                continue
            grown.add(("kw", k))
            try:
                t = time.perf_counter()
                original(*copy.deepcopy(base), **copy.deepcopy(kwargs))
                took = time.perf_counter() - t
            except Exception:
                continue
            for factor in GROW:
                big = bigger(kw_base[k], factor)
                if big is None or len(derived) >= limit or took * 16 > left() / 2:
                    break
                next_kw = dict(kwargs)
                next_kw[k] = big
                took = attempt(base, next_kw)
                if took is None:
                    break
    return derived


def derive(original, cases, budget, limit):
    """Extra test cases: edge cases, then recorded calls with smaller inputs, answered by the original."""
    import copy

    edge_cases = derive_edges(original, cases, budget / 2, EDGE_LIMIT)
    rng = random.Random(0xFA57)
    derived, seen, start = edge_cases, set(), time.perf_counter()
    limit += len(edge_cases)
    for case in cases:
        if "raises" in case:
            continue
        args = dec(case["args"])
        kwargs = {k: dec(v) for k, v in case.get("kwargs", {}).items()}
        for i, arg in enumerate(args):
            for smaller in variants(arg, rng):
                if len(derived) >= limit or time.perf_counter() - start > budget:
                    return derived
                new_args = list(args)
                new_args[i] = smaller
                try:
                    encoded = {"args": enc(new_args), "kwargs": {k: enc(v) for k, v in kwargs.items()}}
                    key = json.dumps(encoded, sort_keys=True)
                    if key in seen:
                        continue
                    seen.add(key)
                    result = original(*copy.deepcopy(new_args), **copy.deepcopy(kwargs))
                    encoded["result"] = enc(result)
                except Exception:
                    continue  # outside the function's domain: not a fair test
                encoded["derived"] = True
                derived.append(encoded)
        for k, v in kwargs.items():
            for smaller in variants(v, rng):
                if len(derived) >= limit or time.perf_counter() - start > budget:
                    return derived
                new_kw = dict(kwargs)
                new_kw[k] = smaller
                try:
                    encoded = {"args": enc(args), "kwargs": {key: enc(val) for key, val in new_kw.items()}}
                    key = json.dumps(encoded, sort_keys=True)
                    if key in seen:
                        continue
                    seen.add(key)
                    result = original(*copy.deepcopy(args), **copy.deepcopy(new_kw))
                    encoded["result"] = enc(result)
                except Exception:
                    continue  # outside the function's domain: not a fair test
                encoded["derived"] = True
                derived.append(encoded)
    return derived


def raises_like(err, name):
    """The port raised the exception the original did. A Rust panic (PanicException, a BaseException)
    never counts: the app's except clauses won't catch it."""
    return isinstance(err, Exception) and type(err).__name__ == name


def best_pass(fn, calls, budget):
    """Fastest time to replay every recorded call once, within a time budget."""
    best, spent, passes = float("inf"), 0.0, 0
    while passes < 3 or (spent < budget and passes < 2000):
        start = time.perf_counter()
        for args, kwargs in calls:
            try:
                fn(*args, **kwargs)
            except (KeyboardInterrupt, SystemExit):
                raise
            except BaseException:  # Rust panics arrive as PanicException
                pass
        took = time.perf_counter() - start
        best, spent, passes = min(best, took), spent + took, passes + 1
    return best


def main():
    mode, job_path = sys.argv[1], sys.argv[2]
    rest = sys.argv[3:]
    if rest[:1] == ["--"]:
        rest = rest[1:]
    with open(job_path, encoding="utf-8") as f:
        job = json.load(f)
    {"profile": cmd_profile, "record": cmd_record, "verify": cmd_verify}[mode](job, rest)


if __name__ == "__main__":
    main()
