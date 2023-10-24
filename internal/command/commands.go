package command

import (
	"github.com/mitchellh/cli"
)

// Commands initializes the Commands factory map.
func Commands(metaPtr *Meta) map[string]cli.CommandFactory {
	if metaPtr == nil {
		metaPtr = new(Meta)
	}
	meta := *metaPtr

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
	}
	return all
}
