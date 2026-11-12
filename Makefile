# Short names for the Web and Desktop application commands. The package.json
# scripts stay the source of truth; docs/development.md documents both.
# `web-bg` and `web-stop` host the same Web server in a tmux session.
.DEFAULT_GOAL := help
.PHONY: help build clean web web-bg web-stop desktop dev-web dev-desktop

PNPM ?= pnpm
ARGS ?=
# Background Web session and its startup log, including the token URL.
WEB_SESSION ?= dsh-web
WEB_LOG ?= /tmp/dsh-web.log

help:
	@echo "make build        pnpm run build           complete repository build"
	@echo "make clean        pnpm run clean           remove build outputs and safe deleted-package residue"
	@echo "make web          pnpm run start:web       serve the built Web artifacts from source"
	@echo "make web-bg       tmux background run      serve the Web UI in session $(WEB_SESSION)"
	@echo "make web-stop     tmux kill-session        stop the background Web session"
	@echo "make desktop      pnpm run start:desktop   launch the built Desktop artifacts"
	@echo "make dev-web      pnpm run dev:web         build, serve, and rebuild Web on source edits"
	@echo "make dev-desktop  pnpm run dev:desktop     build, then launch Desktop"
	@echo "ARGS='--no-open --port 3081' forwards options to the launched application;"
	@echo "the Web commands accept dsh web flags, the Desktop launcher accepts none."

build:
	$(PNPM) run build

clean:
	$(PNPM) run clean

web:
	$(PNPM) run start:web $(ARGS)

web-bg:
	@command -v tmux >/dev/null 2>&1 || { echo "make web-bg: tmux is required"; exit 1; }
	@if tmux has-session -t '$(WEB_SESSION)' 2>/dev/null; then \
		echo "make web-bg: tmux session '$(WEB_SESSION)' already exists"; \
		echo "  attach: tmux attach -t $(WEB_SESSION)"; \
		echo "  stop:   make web-stop"; \
		exit 1; \
	fi
	@mkdir -p "$$(dirname '$(WEB_LOG)')"
	@umask 077; : > '$(WEB_LOG)'; chmod 600 '$(WEB_LOG)' 2>/dev/null || true
	@tmux new-session -d -s '$(WEB_SESSION)' -c '$(CURDIR)' 'umask 077; $(PNPM) run start:web --no-open $(ARGS) > $(WEB_LOG) 2>&1'
	@i=0; while [ $$i -lt 45 ]; do \
		if grep -q '?token=' '$(WEB_LOG)' 2>/dev/null; then break; fi; \
		if ! tmux has-session -t '$(WEB_SESSION)' 2>/dev/null; then break; fi; \
		i=$$((i + 1)); sleep 1; \
	done
	@if tmux has-session -t '$(WEB_SESSION)' 2>/dev/null; then \
		echo "make web-bg: serving in tmux session '$(WEB_SESSION)'"; \
		url=$$(grep -oE 'https?://[^ ]*' '$(WEB_LOG)' 2>/dev/null | tail -1); \
		if [ -n "$$url" ]; then echo "  $$url"; else echo "  URL not in the log yet; run: tail -f $(WEB_LOG)"; fi; \
		echo "  log:    tail -f $(WEB_LOG)"; \
		echo "  attach: tmux attach -t $(WEB_SESSION)"; \
		echo "  stop:   make web-stop"; \
	else \
		echo "make web-bg: the server exited during startup; recent output:"; \
		tail -40 '$(WEB_LOG)' 2>/dev/null || true; \
		exit 1; \
	fi

web-stop:
	@command -v tmux >/dev/null 2>&1 || { echo "make web-stop: tmux is required"; exit 1; }
	@if tmux has-session -t '$(WEB_SESSION)' 2>/dev/null; then \
		tmux kill-session -t '$(WEB_SESSION)'; \
		echo "make web-stop: stopped tmux session '$(WEB_SESSION)'"; \
	else \
		echo "make web-stop: no tmux session '$(WEB_SESSION)'"; \
	fi

desktop:
	$(PNPM) run start:desktop $(ARGS)

dev-web:
	$(PNPM) run dev:web $(ARGS)

dev-desktop:
	$(PNPM) run dev:desktop $(ARGS)
