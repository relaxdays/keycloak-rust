# shellcheck shell=bash

mavenNodejsHook() {
	echo "executing mavenNodejsHook"

	# "pre-install" node+packages so the maven plugin finds and uses these
	mkdir -p js/node node/node_modules
	ln -sf "@node@/bin/node" js/node/node
	ln -sf "@node@/bin/node" node/node
	ln -sf "@node@/bin/npm" node/npm
	ln -sf "@node@/bin/npx" node/npx
	ln -sf "@node@/lib/node_modules/npm" node/node_modules/npm
	ln -sf "@pnpm@/bin/pnpm" node/pnpm
	ln -sf "@pnpm@/libexec/pnpm" node/node_modules/pnpm

	echo "finished mavenNodejsHook"
}

postConfigureHooks+=(mavenNodejsHook)
