package main

import (
	"log"
	"os"

	"github.com/mitchellh/cli"
	"github.com/vssio/go-vss/internal/command"
)

const version = "0.0.1"

func main() {
	os.Exit(run())
}

func run() int {
	metaPtr := new(command.Meta)
	c := &cli.CLI{
		Name:         "vss",
		Version:      version,
		Args:         os.Args[1:],
		Autocomplete: true,
		Commands:     command.Commands(metaPtr),
	}
	exitCode, err := c.Run()
	if err != nil {
		log.Printf("[ERROR] %s", err)
	}
	return exitCode
}
