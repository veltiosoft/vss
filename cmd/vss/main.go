package main

import (
	"os"

	"github.com/vssio/go-vss"
)

func main() {
	os.Exit(vss.Run(os.Args[1:]))
}
