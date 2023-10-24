package command

import (
	"github.com/vssio/go-vss/internal/command/cliconfig"
)

// Meta contains the common fields required by all commands.
type Meta struct {
	Config *cliconfig.Config
}

// SetupConfig initializes the Config field of the Meta struct.
// If the Config field is already initialized, it does nothing.
// NOTE: This method is intended to be called inside a command that requires Config
func (m *Meta) SetupConfig() error {
	if m.Config == nil {
		var err error
		m.Config, err = cliconfig.LoadConfig()
		if err != nil {
			return err
		}
	}
	return nil
}
