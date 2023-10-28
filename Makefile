BIN := vss
VERSION := $$(make -s show-version)
CURRENT_REVISION := $(shell git rev-parse --short HEAD)
BUILD_LDFLAGS := "-s -w -X main.revision=$(CURRENT_REVISION)"
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

.PHONY: docs
docs: build
	cp $(BIN) docs
	cd docs && ./vss build

.PHONY: serve-docs
serve-docs: build
	cp $(BIN) docs
	cd docs && ./vss serve

.PHONY: show-version
show-version: $(GOBIN)/gobump
	@gobump show -r .

$(GOBIN)/gobump:
	@go install github.com/x-motemen/gobump/cmd/gobump@latest

.PHONY: cross
cross: $(GOBIN)/goxz
	goxz -n $(BIN) -pv=v$(VERSION) -build-ldflags=$(BUILD_LDFLAGS) -trimpath cmd/vss/main.go

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
lint: $(GOBIN)/staticcheck
	staticcheck ./...

$(GOBIN)/staticcheck:
	go install honnef.co/go/tools/cmd/staticcheck@latest

.PHONY: fmt
fmt:
	go fmt ./...