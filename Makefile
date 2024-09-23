BIN := vss
VERSION := $$(make -s show-version)
CURRENT_REVISION := $(shell git rev-parse --short HEAD)
BUILD_LDFLAGS := "-s -w -X github.com/veltiosoft/go-vss.revision=$(CURRENT_REVISION)"
GOBIN ?= $(shell go env GOPATH)/bin

.PHONY: tag
tag:
	git tag "v${VERSION}"

.PHONY: release
release: tag
	git push origin "v${VERSION}"

.PHONY: build
build:
	go build -o $(BIN) -ldflags $(BUILD_LDFLAGS) -trimpath cmd/vss/main.go

.PHONY: site
site: build
	cp $(BIN) site
	cd site && ./$(BIN) build

.PHONY: serve-site
serve-site: build
	cp $(BIN) site
	cd site && ./$(BIN) serve

.PHONY: selfupdate
selfupdate: build
	./vss self update

.PHONY: show-version
show-version: $(GOBIN)/gobump
	@gobump show -r .

$(GOBIN)/gobump:
	@go install github.com/x-motemen/gobump/cmd/gobump@latest

.PHONY: cross
cross: $(GOBIN)/goxz
	goxz -n $(BIN) -build-ldflags=$(BUILD_LDFLAGS) -trimpath ./cmd/vss

$(GOBIN)/goxz:
	go install github.com/Songmu/goxz/cmd/goxz@latest

.PHONY: test
test: build
	go test -shuffle=on -v ./...

.PHONY: bench 
bench:
	go test -v -bench=. -benchmem ./...

.PHONY: clean
clean:
	rm -rf $(BIN) goxz
	go clean

.PHONY: lint
lint: $(GOBIN)/golangci-lint
	golangci-lint run ./...

$(GOBIN)/golangci-lint:
	go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

.PHONY: fmt
fmt:
	go fmt ./...
