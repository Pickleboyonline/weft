# weft development tasks. Run `make` to list them.

EXT     := editors/vscode
VSIX    := $(EXT)/weft-language.vsix
EXT_SRC := $(EXT)/package.json $(EXT)/language-configuration.json $(EXT)/README.md \
           $(wildcard $(EXT)/syntaxes/*.json) $(wildcard $(EXT)/icons/*.svg)

.DEFAULT_GOAL := help
.PHONY: help vscode vscode-test vscode-preview vscode-package vscode-uninstall

help: ## List tasks
	@awk 'BEGIN { FS = ":.*## " } /^[a-z-]+:.*## / { printf "  make %-17s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

vscode: vscode-test $(VSIX) ## Test, package and install the VS Code extension
	code --install-extension $(VSIX) --force
	@echo "Installed. Reload VS Code (Developer: Reload Window) to pick it up."

vscode-test: $(EXT)/node_modules ## Run the grammar tests
	cd $(EXT) && npm test

vscode-preview: $(EXT)/node_modules ## Render .weft files to editors/vscode/out/preview.html (FILES=app.weft to filter)
	cd $(EXT) && npm run preview -- $(FILES)

vscode-package: $(VSIX) ## Build editors/vscode/weft-language.vsix

vscode-uninstall: ## Remove the extension from VS Code
	code --uninstall-extension weft.weft-language

# Re-package only when the grammar, config or manifest changes.
$(VSIX): $(EXT_SRC) $(EXT)/node_modules
	cd $(EXT) && npm run package

$(EXT)/node_modules: $(EXT)/package.json $(EXT)/package-lock.json
	cd $(EXT) && npm install --no-audit --no-fund
	@touch $@
