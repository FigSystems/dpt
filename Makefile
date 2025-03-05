DESTDIR ?= /usr/local

none-specified: build-release

install: install-release

build-debug:
	cargo build

build-release:
	cargo build --release

install-makefpkg:
	mkdir -p $(DESTDIR)/bin
	cp tools/makefpkg $(DESTDIR)/bin/makefpkg
	chmod +x $(DESTDIR)/bin/makefpkg

install-debug: build-debug install-makefpkg
	mkdir -p $(DESTDIR)/bin
	cp -f target/x86_64-unknown-linux-musl/debug/fpkg $(DESTDIR)/bin/
	chown root:root $(DESTDIR)/bin/fpkg
	chmod u+s $(DESTDIR)/bin/fpkg

install-release: build-release install-makefpkg
	build --release
	mkdir -p $(DESTDIR)/bin
	cp -f target/x86_64-unknown-linux-musl/release/fpkg $(DESTDIR)/bin/
	chown root:root $(DESTDIR)/bin/fpkg
	chmod u+s $(DESTDIR)/bin/fpkg
