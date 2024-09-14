package vss

import (
	"github.com/mitchellh/cli"
)

// Commands initializes the Commands factory map.
func initCommands(metaPtr *Meta) map[string]cli.CommandFactory {
	if metaPtr == nil {
		metaPtr = new(Meta)
	}
	meta := *metaPtr
	meta.Version = version
	meta.Revision = revision

	all := map[string]cli.CommandFactory{
		"build": func() (cli.Command, error) {
			return &BuildCommand{
				Meta: meta,
			}, nil
		},
		"serve": func() (cli.Command, error) {
			return &ServeCommand{
				Meta: meta,
			}, nil
		},
		"new": func() (cli.Command, error) {
			return &NewCommand{
				Meta: meta,
			}, nil
		},
		"self update": func() (cli.Command, error) {
			return &SelfUpdateCommand{
				Meta: meta,
			}, nil
		},
		"self version": func() (cli.Command, error) {
			return &SelfVersionCommand{
				Meta: meta,
			}, nil
		},
	}
	return all
}
