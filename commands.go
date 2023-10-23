package vss

import (
	"github.com/mitchellh/cli"
	"github.com/vssio/go-vss/internal/command"
)

// Commands is a map of available commands.
var Commands map[string]cli.CommandFactory

// initCommands initializes the Commands map.
func initCommands() {
	Commands = map[string]cli.CommandFactory{
		"build": func() (cli.Command, error) {
			return &command.BuildCommand{}, nil

		},
		"serve": func() (cli.Command, error) {
			return &command.ServeCommand{}, nil
		},
	}
}
