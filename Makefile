DESTDIR ?= /usr/local

none-specified: build-release

install: install-release

build-debug:
	cargo build

build-release:
	cargo build --release

install-makedpt:
	mkdir -p $(DESTDIR)/bin
	cp tools/makedpt $(DESTDIR)/bin/makedpt
	chmod +x $(DESTDIR)/bin/makedpt

install-debug: install-makedpt
	mkdir -p $(DESTDIR)/bin
	cp -f target/x86_64-unknown-linux-musl/debug/dpt $(DESTDIR)/bin/
	chown root:root $(DESTDIR)/bin/dpt
	chmod u+s $(DESTDIR)/bin/dpt

install-release: install-makedpt
	mkdir -p $(DESTDIR)/bin
	cp -f target/x86_64-unknown-linux-musl/release/dpt $(DESTDIR)/bin/
	chown root:root $(DESTDIR)/bin/dpt
	chmod u+s $(DESTDIR)/bin/dpt
