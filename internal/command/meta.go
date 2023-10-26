package command

import "github.com/vssio/go-vss/internal/config"

// Meta contains the common fields required by all commands.
type Meta struct {
	Config *config.Config
}

// SetupConfig initializes the Config field of the Meta struct.
// If the Config field is already initialized, it does nothing.
// NOTE: This method is intended to be called inside a command that requires Config
func (m *Meta) SetupConfig() error {
	if m.Config == nil {
		var err error
		m.Config, err = config.LoadConfig()
		if err != nil {
			return err
		}
		m.Config.Dist = "dist"
		m.Config.Static = "static"
		m.Config.Layouts = "layouts"
	}
	return nil
}
