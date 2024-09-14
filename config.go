package vss

import (
	"log"
	"os"
	"path/filepath"

	"github.com/pelletier/go-toml/v2"
)

type GoldMarkHighlightConfig struct {
	Style       *string
	WithNumbers *bool `toml:"with_numbers"`
}

// GoldMarkConfig is a struct for configuring goldmark.
// なぜ Pointer なのか？
// config file にキーを設定したかどうかを判定するため
type GoldMarkConfig struct {
	HighlightConfig *GoldMarkHighlightConfig `toml:"highlight"`
}

type BuildConfig struct {
	IgnoreFiles []string
	Goldmark    GoldMarkConfig `toml:"goldmark"`
}

type Config struct {
	Build           BuildConfig `toml:"build"`
	SiteTitle       string      `toml:"site_title"`
	SiteDescription string      `toml:"site_description"`
	BaseUrl         string      `toml:"base_url"`

	// The following settings are not in config.toml
	Dist    string // dist directory
	Static  string // static directory
	Layouts string // layouts directory
}

// LoadConfig loads a TOML text into a Config struct.
func LoadConfig() (*Config, error) {
	path, err := cliConfigFile()
	if err != nil {
		return nil, err
	}
	c, err := loadConfigFile(path)
	if err != nil {
		return nil, err
	}
	// set default values
	c.Dist = "dist"
	c.Static = "static"
	c.Layouts = "layouts"
	return c, nil
}

func loadConfigFile(path string) (*Config, error) {
	log.Printf("[INFO] Loading config file: %s", path)
	d, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	config := &Config{}
	err = toml.Unmarshal(d, config)
	if err != nil {
		return nil, err
	}
	return config, nil
}

// AsMap returns a map[string]interface{} representation of the Config struct.
func (c *Config) AsMap() map[string]interface{} {
	return map[string]interface{}{
		"site_title":       c.SiteTitle,
		"site_description": c.SiteDescription,
		"base_url":         c.BaseUrl,
	}
}

func configFile() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, "vss.toml"), nil
}

// cliConfigFileOverride returns the value of VSS_CONFIG_FILE if set.
func cliConfigFileOverride() string {
	return os.Getenv("VSS_CONFIG_FILE")
}

func cliConfigFile() (string, error) {
	configFilePath := cliConfigFileOverride()
	if configFilePath == "" {
		var err error
		configFilePath, err = configFile()
		if err != nil {
			return "", err
		}
	}

	f, err := os.Open(configFilePath)
	if err == nil {
		f.Close()
		return configFilePath, nil
	} else {
		return "", err
	}
}
