package skeleton

import (
	"embed"
	"errors"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"strings"
)

//go:embed assets/*
var assets embed.FS

func GenerateSkeleton(distDir string) error {
	if err := copyEmbedFiles(&assets, distDir); err != nil {
		return err
	}

	// create static directory
	err := os.MkdirAll(filepath.Join(distDir, "static"), os.ModePerm)
	if err != nil {
		return err
	}
	return nil
}

// copyEmbedFiles copies all files in the embed.FS to the destination directory.
func copyEmbedFiles(efs *embed.FS, distDir string) error {
	// Create the destination directory if it doesn't exist
	if err := createDistDir(distDir); err != nil {
		return err
	}

	// Walk through the embed.FS and copy all files to the destination directory
	if err := fs.WalkDir(efs, "assets", func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}

		if !d.IsDir() {
			// Get the destination path by removing the "assets/" prefix
			distPath := filepath.Join(distDir, strings.Replace(path, "assets/", "", 1))

			// Create the destination directory if it doesn't exist
			if err := os.MkdirAll(filepath.Dir(distPath), 0755); err != nil {
				return err
			}

			// Copy the file to the destination directory
			data, err := efs.ReadFile(path)
			if err != nil {
				return err
			}
			if err := os.WriteFile(distPath, data, 0644); err != nil {
				return err
			}
		}

		return nil
	}); err != nil {
		return err
	}

	return nil
}

func createDistDir(dist string) error {
	// TODO: cache dist directory
	if existDir(dist) {
		return errors.New("dist directory already exists")
	} else {
		log.Printf("[INFO] creating dist directory: %s", dist)
		if err := os.MkdirAll(dist, os.ModePerm); err != nil {
			return err
		}
	}
	return nil
}

// existDir checks if a directory exists.
func existDir(dir string) bool {
	info, err := os.Stat(dir)
	if os.IsNotExist(err) {
		return false
	}
	return info.IsDir()
}
