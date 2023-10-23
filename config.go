package vss

import (
	"github.com/pelletier/go-toml/v2"
)

type BuildConfig struct {
	IgnoreFiles []string
}

type Config struct {
	Build      BuildConfig
	Title      string
	Descrition string
	BaseUrl    string
}

// LoadConfig loads a TOML text into a Config struct.
func LoadConfig(toml_text string) (Config, error) {
	var config Config
	err := toml.Unmarshal([]byte(toml_text), &config)
	if err != nil {
		return Config{}, err
	}
	return config, nil
}

// AsMap returns a map[string]interface{} representation of the Config struct.
func (c *Config) AsMap() map[string]interface{} {
	return map[string]interface{}{
		"build": map[string]interface{}{
			"ignore_files": c.Build.IgnoreFiles,
		},
		"title":       c.Title,
		"description": c.Descrition,
		"base_url":    c.BaseUrl,
	}
}
