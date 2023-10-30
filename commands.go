package vss

import (
	"github.com/mitchellh/cli"
	"github.com/vssio/go-vss/internal/command"
)

// Commands initializes the Commands factory map.
func initCommands(metaPtr *command.Meta) map[string]cli.CommandFactory {
	if metaPtr == nil {
		metaPtr = new(command.Meta)
	}
	meta := *metaPtr
	meta.Version = version

	all := map[string]cli.CommandFactory{
		"build": func() (cli.Command, error) {
			return &command.BuildCommand{
				Meta: meta,
			}, nil

		},
		"serve": func() (cli.Command, error) {
			return &command.ServeCommand{
				Meta: meta,
			}, nil
		},
		"new": func() (cli.Command, error) {
			return &command.NewCommand{
				Meta: meta,
			}, nil
		},
	}
	return all
}
