BIN := vss
VERSION := $$(make -s show-version)
CURRENT_REVISION := $(shell git rev-parse --short HEAD)
BUILD_LDFLAGS := "-s -w -X main.revision=$(CURRENT_REVISION)"
GOBIN ?= $(shell go env GOPATH)/bin

.PHONY: tag
tag:
	git tag "v${VERSION}"
	git push origin "v${VERSION}"

.PHONY: build
build:
	go build -o $(BIN) -ldflags $(BUILD_LDFLAGS) -trimpath cmd/vss/main.go

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
	go test -v ./...

.PHONY: clean
clean:
	rm -rf $(BIN) goxz
	go clean

.PHONY: upload
upload: $(GOBIN)/ghr
	ghr "v$(VERSION)" goxz

$(GOBIN)/ghr:
	go install github.com/tcnksm/ghr@latest

.PHONY: lint
lint: $(GOBIN)/staticcheck
	staticcheck ./...

$(GOBIN)/staticcheck:
	go install honnef.co/go/tools/cmd/staticcheck@latest

.PHONY: fmt
fmt:
	go fmt ./...