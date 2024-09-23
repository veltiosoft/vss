package main

import (
	"os"

	"github.com/veltiosoft/go-vss"
)

func main() {
	os.Exit(vss.Run(os.Args[1:]))
}
