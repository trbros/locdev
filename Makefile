# LocDev — atalhos de desenvolvimento. Rode `make` para ver a lista.
#
# Sandbox: acrescente SANDBOX=1 a qualquer alvo para usar uma home isolada em
# .dev/home (config, CA, logs e socket separados da sua instalação real):
#   make demo SANDBOX=1
# As portas (80/443/10053) são as mesmas: com o daemon real rodando, a
# sandbox cai para 7080/7443 e fica sem DNS. Rode `make stop` antes, se quiser.

SHELL := /bin/bash
.DEFAULT_GOAL := help

CLI     := ./target/debug/locdev
DESKTOP := apps/desktop
DOCS    := apps/docs
DEV     := $(CURDIR)/.dev
DEMO    := $(DEV)/demo
OPEN    := $(shell command -v xdg-open || command -v open)

ifdef SANDBOX
export LOCDEV_HOME := $(DEV)/home
endif

# O terminal do VS Code em snap exporta caminhos de GTK/GIO do snap; o app
# carrega as bibliotecas do snap e cai com "symbol lookup error" (libpthread).
ifdef SNAP
APP_ENV := env -u SNAP -u GTK_PATH -u GTK_EXE_PREFIX -u GTK_IM_MODULE_FILE -u GIO_MODULE_DIR \
	-u GSETTINGS_SCHEMA_DIR -u GDK_PIXBUF_MODULE_FILE -u GDK_PIXBUF_MODULEDIR -u LOCPATH \
	$(if $(XDG_DATA_DIRS_VSCODE_SNAP_ORIG),XDG_DATA_DIRS=$(XDG_DATA_DIRS_VSCODE_SNAP_ORIG))
endif

## ── Geral ─────────────────────────────────────────────────────────────────

.PHONY: help
help: ## Lista os comandos
	@awk 'BEGIN {FS = ":.*## "} \
		/^## ──/ { sub(/^## /, ""); printf "\n\033[2m%s\033[0m\n", $$0; next } \
		/^[a-zA-Z0-9_-]+:.*## / { printf "  \033[33m%-18s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@printf "\n  \033[2mSANDBOX=1 isola tudo em .dev/home · SITE=<nome> para logs/up/down\033[0m\n\n"

.PHONY: setup
setup: ## Instala as dependências JS do app e da documentação
	cd $(DESKTOP) && pnpm install
	cd $(DOCS) && pnpm install

.PHONY: build
build: ## Compila a CLI e o daemon (debug)
	cargo build -p locdev -p locdev-daemon

.PHONY: release
release: ## Compila a CLI e o daemon (release)
	cargo build --release -p locdev -p locdev-daemon

## ── Qualidade ─────────────────────────────────────────────────────────────

.PHONY: test
test: ## Roda os testes Rust
	cargo test

.PHONY: cli-catalog
cli-catalog: ## Regenera os catálogos de comandos e frameworks e o changelog (app e docs)
	LOCDEV_UPDATE_CATALOG=1 cargo test -p locdev catalog
	LOCDEV_UPDATE_CATALOG=1 cargo test -p locdev-frameworks committed_definitions
	LOCDEV_UPDATE_CATALOG=1 cargo test -p locdev-scaffold committed_starters
	node $(DOCS)/scripts/sync-changelogs.mjs

.PHONY: third-party
third-party: ## Regenera a lista de dependências de terceiros e licenças (Configurações → Sobre)
	cd $(DESKTOP) && node scripts/third-party.mjs

.PHONY: docs-test
docs-test: ## Testes do site (conteúdo, traduções, rotas, comparativo, changelog)
	@test -f $(DOCS)/.env || cp $(DOCS)/.env.example $(DOCS)/.env
	cd $(DOCS) && pnpm test && pnpm check:comparison && pnpm check:routes

.PHONY: test-ui
test-ui: ## Percorre todas as telas do app contra o daemon simulado (Playwright)
	cd $(DESKTOP) && pnpm test:ui

.PHONY: test-a11y
test-a11y: ## Acessibilidade (axe, WCAG 2.2 AA, temas claro e escuro, foco por teclado) em todas as telas
	cd $(DESKTOP) && pnpm test:a11y

.PHONY: lint
lint: ## clippy (Rust) + typecheck (app e docs) + traduções do app
	cargo clippy --all-targets -- -D warnings
	cd $(DESKTOP) && pnpm typecheck && pnpm test:i18n && node scripts/third-party.mjs --check
	node $(DOCS)/scripts/sync-changelogs.mjs --check
	cd $(DOCS) && pnpm typecheck

.PHONY: fmt
fmt: ## Formata Rust e TypeScript
	cargo fmt --all
	cd $(DESKTOP) && pnpm format
	cd $(DOCS) && pnpm exec biome format --write .

.PHONY: check
check: ## Tudo que o CI roda: formatação, lint e testes
	cargo fmt --all --check
	cd $(DOCS) && pnpm exec biome check .
	$(MAKE) lint test docs-test

## ── Daemon e CLI ──────────────────────────────────────────────────────────

.PHONY: start stop restart status doctor list version
start: build ## Sobe o daemon em segundo plano
	$(CLI) start
stop: build ## Para o daemon (e todos os sites)
	$(CLI) stop
restart: build ## Reinicia o daemon (pega o build novo)
	$(CLI) restart
status: build ## Estado do daemon, proxy, DNS e HTTPS
	$(CLI) status
doctor: build ## Diagnóstico do ambiente
	$(CLI) doctor
list: build ## Lista os sites
	$(CLI) list
version: build ## Versões da CLI e do daemon
	$(CLI) version

.PHONY: logs up down
logs: build ## Saída de um site ao vivo: make logs SITE=demo
	@test -n "$(SITE)" || (echo "use: make logs SITE=<nome>"; exit 1)
	$(CLI) logs $(SITE) -f
up: build ## Inicia um site: make up SITE=demo
	@test -n "$(SITE)" || (echo "use: make up SITE=<nome>"; exit 1)
	$(CLI) up $(SITE)
down: build ## Para um site: make down SITE=demo
	@test -n "$(SITE)" || (echo "use: make down SITE=<nome>"; exit 1)
	$(CLI) down $(SITE)

.PHONY: demo demo-clean
demo: build ## Cria um projeto Node de exemplo, registra, inicia e testa pelo proxy
	@mkdir -p $(DEMO)
	@printf '{"name":"demo","scripts":{"dev":"node server.js"}}\n' > $(DEMO)/package.json
	@printf "require('http').createServer((q,r)=>r.end('hello from demo '+q.headers.host+'\\\\n')).listen(process.env.PORT,'127.0.0.1',()=>console.log('listening on',process.env.PORT));\n" > $(DEMO)/server.js
	$(CLI) start
	@$(CLI) list | grep -q 'demo.test' || $(CLI) add $(DEMO) --name demo
	$(CLI) up demo
	@sleep 1.5
	@# Use the URLs the daemon reports: ports fall back (7080/7443) when 80/443 are taken.
	@json=$$($(CLI) --json list); \
	url=$$(echo "$$json" | grep -o '"url": *"[^"]*demo[^"]*"' | head -1 | cut -d'"' -f4); \
	https=$$(echo "$$json" | grep -o '"https_url": *"[^"]*"' | head -1 | cut -d'"' -f4); \
	probe() { printf "  %-34s " "$$1"; curl -s $$2 -m 3 -w ' [%{http_code}]' "$$1" | head -c 120 | tr -d '\n'; echo; }; \
	echo; echo "── respostas (primeira linha):"; \
	probe "$$url"; \
	probe "http://demo.test$${url##*localhost}"; \
	[ -n "$$https" ] && probe "$$https" -k; \
	echo; echo "   .test precisa de make dns-install; https sem -k precisa de make tls-trust"
	@echo; echo "Acompanhe com: make logs SITE=demo"
demo-clean: build ## Remove o site de exemplo (os arquivos em .dev/demo ficam)
	-$(CLI) rm demo

## ── Sistema (pedem confirmação e senha) ───────────────────────────────────

.PHONY: dns-status dns-install dns-uninstall tls-status tls-trust tls-untrust
dns-status: build ## Como o sistema resolve *.test
	$(CLI) dns status
dns-install: build ## Roteia *.test para o LocDev (mostra o script antes)
	$(CLI) dns install
dns-uninstall: build ## Desfaz o dns-install
	$(CLI) dns uninstall
tls-status: build ## HTTPS, CA local e confiança
	$(CLI) tls status
tls-trust: build ## Confia a CA local no sistema e navegadores (mostra o plano antes)
	$(CLI) tls trust
tls-untrust: build ## Remove a confiança na CA
	$(CLI) tls untrust

.PHONY: service-status service-install service-uninstall
service-status: build ## Se o daemon inicia no login
	$(CLI) startup status
service-install: release ## Daemon no login (usa o build release)
	LOCDEV_DAEMON=$(CURDIR)/target/release/locdevd ./target/release/locdev startup install
service-uninstall: build ## Remove o daemon do login
	$(CLI) startup uninstall

## ── App desktop ───────────────────────────────────────────────────────────

.PHONY: app app-mock app-build sidecars screenshots
app: ## Abre o app de verdade (Tauri dev, ligado ao daemon real)
	@pkg-config --exists webkit2gtk-4.1 2>/dev/null || [ "$$(uname)" != Linux ] || \
		(echo "falta o WebKitGTK: sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev"; exit 1)
	cd $(DESKTOP) && $(APP_ENV) pnpm tauri dev
app-mock: ## Abre a interface no navegador com daemon simulado (http://localhost:1420)
	@(sleep 2 && $(OPEN) http://localhost:1420 >/dev/null 2>&1 &) ; cd $(DESKTOP) && pnpm dev
app-build: ## Gera os instaladores (target/release/bundle); assinados para o atualizador se houver .updater-keys/
	@if [ -f .updater-keys/locdev-updater.key ]; then \
		cd $(DESKTOP) && TAURI_SIGNING_PRIVATE_KEY="$$(cat ../../.updater-keys/locdev-updater.key)" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm tauri build; \
	else \
		cd $(DESKTOP) && pnpm tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'; \
	fi
sidecars: ## Compila locdevd/locdev como sidecars do app (debug)
	cd $(DESKTOP) && node scripts/prepare-sidecars.mjs --debug
screenshots: ## Regenera as capturas do app usadas na documentação
	cd $(DESKTOP) && pnpm screenshots

## ── Documentação ──────────────────────────────────────────────────────────

.PHONY: docs docs-build docs-preview
docs: ## Abre o site e a documentação em modo dev (http://localhost:1440)
	@test -f $(DOCS)/.env || cp $(DOCS)/.env.example $(DOCS)/.env
	@(sleep 4 && $(OPEN) http://localhost:1440 >/dev/null 2>&1 &) ; cd $(DOCS) && pnpm dev -p 1440
docs-build: ## Build de produção do site (Next.js, em apps/docs/.next)
	@test -f $(DOCS)/.env || cp $(DOCS)/.env.example $(DOCS)/.env
	cd $(DOCS) && pnpm build
docs-preview: docs-build ## Serve o build do site (http://localhost:1441)
	@(sleep 2 && $(OPEN) http://localhost:1441 >/dev/null 2>&1 &) ; cd $(DOCS) && pnpm start -p 1441

## ── Limpeza ───────────────────────────────────────────────────────────────

.PHONY: clean
clean: ## Remove builds e a sandbox (.dev); não toca na sua instalação real
	cargo clean
	rm -rf $(DEV) $(DESKTOP)/dist $(DESKTOP)/src-tauri/binaries $(DOCS)/.next
