PREFIX ?= /usr/local
DESTDIR ?=

.PHONY: install install-system uninstall patch-dkms

install: install-system

install-system:
	sudo -E env "PATH=$$PATH" ./scripts/install-system.sh

uninstall:
	sudo ./scripts/uninstall.sh

patch-dkms:
	sudo ./scripts/apply-dmi-patch.sh
