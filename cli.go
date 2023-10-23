package vss

import (
	"os"

	"github.com/mitchellh/cli"
)

const (
	version = "0.0.1"
	name    = "vss"
)

// NewCLI returns a new CLI object.
func NewCLI() *cli.CLI {
	c := cli.NewCLI(name, version)
	c.Args = os.Args[1:]

	initCommands()
	c.Commands = Commands
	return c
}
