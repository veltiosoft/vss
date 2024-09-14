package vss

import (
	"log"

	"github.com/mitchellh/cli"
)

const version = "0.11.0"

var revision = "HEAD"

func Version() string {
	return version
}

func Run(args []string) int {
	metaPtr := new(Meta)
	c := &cli.CLI{
		Name:         "vss",
		Version:      version,
		Args:         args,
		Autocomplete: true,
		Commands:     initCommands(metaPtr),
	}
	exitCode, err := c.Run()
	if err != nil {
		log.Printf("[ERROR] %s", err)
	}
	return exitCode
}
