.PHONY: docker docker-test docker-rebuild
REVISION:=`git rev-parse HEAD`
VERSION:=latest

all:

docker: clean
	docker build -t proto2:$(VERSION) .

docker-test: clean
	docker run -t -v "$$PWD:/proto2" proto2:$(VERSION) cargo test

docker-rebuild: clean
	docker run -t -v "$$PWD:/proto2" proto2:$(VERSION) cargo build

docker-run: clean
	docker run -t -v "$$PWD:/proto2" proto2:$(VERSION) cargo run

run:
	docker run -ti -p 8080:8080 -v "$$PWD:/proto2" proto2:$(VERSION) bash

build:
	cargo build

test:
	cargo test

clean:
	cargo clean
