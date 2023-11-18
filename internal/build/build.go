package build

import (
	"bytes"
	"errors"
	"io"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/cbroglie/mustache"
	"github.com/vssio/go-vss/internal/config"
	"github.com/yuin/goldmark"
	"github.com/yuin/goldmark/extension"
)

// Builder is a struct for building a static site.
type Builder struct {
	config *config.Config

	// init in Run()
	templateMap map[string]*mustache.Template
}

// NewBuilder returns a new Builder.
func NewBuilder(config *config.Config) *Builder {
	return &Builder{
		config: config,
	}
}

func (b *Builder) Run() error {
	if err := createDistDir(b.config.Dist); err != nil {
		return err
	}

	log.Printf("[INFO] copying static files from %s to %s\n", b.config.Static, b.config.Dist)
	if err := copyStatic(b.config.Static, b.config.Dist); err != nil {
		return err
	}

	markdownFiles, err := getFilePathsByExt(".", ".md")
	if err != nil {
		return err
	}
	log.Printf("[INFO] found %d markdown files\n", len(markdownFiles))

	templateFiles, err := getFilePathsByExt(b.config.Layouts, ".html")
	if err != nil {
		return err
	}
	if err := b.initTemplateMap(templateFiles); err != nil {
		return err
	}

	log.Printf("[INFO] rendering markdown files\n")
	gm := initGoldmark()
	// for storing rendered html
	var buf bytes.Buffer
	renderContext := b.config.AsMap()
	for _, markdownPath := range markdownFiles {
		log.Printf("[INFO] rendering %s\n", markdownPath)
		markdown, err := os.ReadFile(markdownPath)
		if err != nil {
			return err
		}
		if err := gm.Convert(markdown, &buf); err != nil {
			return err
		}
		renderContext["contents"] = buf.String()
		buf.Reset()

		htmlPath := convertMarkdownPathToHtmlPath(markdownPath)
		distFile, err := createDistFile(filepath.Join(b.config.Dist, htmlPath))
		if err != nil {
			return err
		}
		defer distFile.Close()
		template, err := b.lookUpTemplate(htmlPath)
		if err != nil {
			return err
		}
		template.FRender(distFile, renderContext)
	}
	return nil
}

func (b *Builder) initTemplateMap(templateFiles []string) error {
	m := make(map[string]*mustache.Template, len(templateFiles))
	for _, templateFile := range templateFiles {
		t, err := mustache.ParseFile(templateFile)
		if err != nil {
			return err
		}
		m[templateFile] = t
	}
	b.templateMap = m
	return nil
}

// lookUpTemplate returns the path (file path) of the template path.
func (b *Builder) lookUpTemplate(path string) (*mustache.Template, error) {
	dir := filepath.Dir(path)
	layoutsDir := b.config.Layouts

	t, ok := b.templateMap[filepath.Join(layoutsDir, path)]
	if ok {
		return t, nil
	}
	t, ok = b.templateMap[filepath.Join(layoutsDir, dir, "default.html")]
	if ok {
		return t, nil
	}
	t, ok = b.templateMap[filepath.Join(layoutsDir, "default.html")]
	if ok {
		return t, nil
	}
	return nil, errors.New("template not found")
}

func replaceExt(filePath, from, to string) string {
	ext := filepath.Ext(filePath)
	if len(from) > 0 && strings.ToLower(ext) != from {
		return filePath
	}
	return filePath[:len(filePath)-len(ext)] + to
}

func convertMarkdownPathToHtmlPath(markdownPath string) string {
	// TODO: support `markdown` extension ?
	return replaceExt(markdownPath, ".md", ".html")
}

// copyStatic copy all files in the static directory (src) to the dist directory.
func copyStatic(src, dist string) error {
	if existDir(src) {
		// Create destination directory if it does not exist
		if err := os.MkdirAll(dist, os.ModePerm); err != nil {
			return err
		}

		// Get all files in the source directory
		files, err := os.ReadDir(src)
		if err != nil {
			return err
		}

		// Copy each file to the destination directory
		for _, file := range files {
			srcFile := filepath.Join(src, file.Name())
			distFile := filepath.Join(dist, file.Name())

			if file.IsDir() {
				// Recursively copy subdirectories
				if err := copyStatic(srcFile, distFile); err != nil {
					return err
				}
			} else {
				// Copy file contents
				if err := copyFile(srcFile, distFile); err != nil {
					return err
				}
			}
		}
	} else {
		log.Printf("[INFO] static directory not found. skip copying static files.")
	}

	return nil
}

// copyFile copies a file from src to dst.
func copyFile(src, dst string) error {
	srcFile, err := os.Open(src)
	if err != nil {
		return err
	}
	defer srcFile.Close()

	dstFile, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer dstFile.Close()

	_, err = io.Copy(dstFile, srcFile)
	if err != nil {
		return err
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

func createDistDir(dist string) error {
	// TODO: cache dist directory
	if existDir(dist) {
		log.Printf("[INFO] re creating dist directory: %s", dist)
		if err := os.RemoveAll(dist); err != nil {
			return err
		}
		if err := os.Mkdir(dist, os.ModePerm); err != nil {
			return err
		}
	} else {
		log.Printf("[INFO] creating dist directory: %s", dist)
		if err := os.Mkdir(dist, os.ModePerm); err != nil {
			return err
		}
	}
	return nil
}

func getFilePathsByExt(dirPath, ext string) ([]string, error) {
	var filePaths []string

	err := filepath.Walk(dirPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		if !info.IsDir() && strings.HasSuffix(info.Name(), ext) {
			filePaths = append(filePaths, path)
		}

		return nil
	})

	if err != nil {
		return nil, err
	}

	return filePaths, nil
}

func initGoldmark() goldmark.Markdown {
	return goldmark.New(
		goldmark.WithExtensions(extension.GFM),
	)
}

func createDistFile(dist string) (*os.File, error) {
	dir := filepath.Dir(dist)
	if !existDir(dir) {
		os.MkdirAll(dir, os.ModePerm)
	}
	return os.Create(dist)
}
