package build

import (
	"bytes"
	"errors"
	"io"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/adrg/frontmatter"
	chromahtml "github.com/alecthomas/chroma/v2/formatters/html"
	"github.com/cbroglie/mustache"
	"github.com/vssio/go-vss/internal/config"
	"github.com/yuin/goldmark"
	highlighting "github.com/yuin/goldmark-highlighting/v2"
	"github.com/yuin/goldmark/extension"
)

// Builder is a struct for building a static site.
type Builder struct {
	config *config.Config

	// init in Run()
	templateMap       map[string]*mustache.Template
	gm                goldmark.Markdown
	buf               bytes.Buffer
	baseRenderContext map[string]interface{}
}

// NewBuilder returns a new Builder.
func NewBuilder(config *config.Config) *Builder {
	return &Builder{
		config: config,
	}
}

// GetDistPath returns the dist directory path.
func (b Builder) GetDistPath() string {
	return b.config.Dist
}

// ReloadConfig reloads the config file.
func (b *Builder) ReloadConfig() error {
	c, err := config.LoadConfig()
	if err != nil {
		return err
	}
	b.config = c
	return nil
}

// Run builds the static site.
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
	b.gm = b.initGoldmark()
	// for storing rendered html
	b.baseRenderContext = b.config.AsMap()
	for _, markdownPath := range markdownFiles {
		log.Printf("[INFO] rendering %s\n", markdownPath)
		if err := b.renderContent(markdownPath); err != nil {
			return err
		}
	}
	return nil
}

// renderContent renders the markdown file and writes the result to the dist directory.
func (b *Builder) renderContent(markdownPath string) error {
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

	filedata, err := b.getFileData(markdownPath)
	if err != nil {
		return err
	}

	// postSlug 処理
	// TODO: ユーザー的に不要かもなのでどっかで消すか判断する
	if filedata.FrontMatter.PostSlug == "" {
		filedata.FrontMatter.PostSlug = filepath.ToSlash(strings.TrimSuffix(htmlPath, ".html"))
	}

	// og image 処理
	if filedata.FrontMatter.OgImage == "" && filedata.FrontMatter.Emoji != "" {
		svgPath := replaceExt(markdownPath, ".md", ".svg")
		imagePath := filepath.Join(b.config.Dist, svgPath)
		file, err := os.Create(imagePath)
		if err != nil {
			return err
		}
		defer file.Close()
		if err := filedata.FrontMatter.SaveTwemojiSvg(file); err != nil {
			return err
		}
		filedata.FrontMatter.OgImage = "/" + filepath.ToSlash(svgPath)
	}

	renderContext, err := b.getRenderContext(filedata)
	if err != nil {
		return err
	}
	return template.FRender(distFile, renderContext)
}

func (b *Builder) getFileData(markdownPath string) (FileData, error) {
	var filedata FileData
	defer b.buf.Reset()
	content, err := os.ReadFile(markdownPath)
	if err != nil {
		return filedata, err
	}
	var yfm YamlFrontMatter
	markdown, err := frontmatter.Parse(strings.NewReader(string(content)), &yfm)
	if err != nil {
		return filedata, err
	}
	if err := b.gm.Convert(markdown, &b.buf); err != nil {
		return filedata, err
	}
	filedata.Content = b.buf.String()

	// content と markdown が同じ場合は frontmatter がないとみなし、ここで終了
	if bytes.Equal(content, markdown) {
		return filedata, nil
	}
	filedata.FrontMatter = yfm
	return filedata, nil
}

// getRenderContext returns a map[string]interface{} that contains the content of the markdown file.
func (b *Builder) getRenderContext(filedata FileData) (map[string]interface{}, error) {
	renderContext := b.baseRenderContext
	renderContext["contents"] = filedata.Content

	// matter のフィールドを renderContext に追加
	for k, v := range filedata.FrontMatter.AsMap() {
		renderContext[k] = v
	}
	return renderContext, nil
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

func (b *Builder) initGoldmark() goldmark.Markdown {
	// TODO: highlight は option にする(例: 他の syntax highlighter を使いたい場合のため)
	extensions := []goldmark.Extender{
		// default extensions
		extension.GFM,
	}
	highlightoptions := []highlighting.Option{}
	if b.config.Build.Goldmark.HighlightConfig != nil {
		if b.config.Build.Goldmark.HighlightConfig.Style != nil {
			highlightoptions = append(highlightoptions, highlighting.WithStyle(*b.config.Build.Goldmark.HighlightConfig.Style))
		}
		// TODO: キーがない場合は highlight しないようにする
		if b.config.Build.Goldmark.HighlightConfig.WithNumbers != nil {
			highlightoptions = append(
				highlightoptions,
				highlighting.WithFormatOptions(chromahtml.WithLineNumbers(*b.config.Build.Goldmark.HighlightConfig.WithNumbers)),
			)
		}
	}

	if len(highlightoptions) > 0 {
		extensions = append(extensions, highlighting.NewHighlighting(highlightoptions...))
	}
	return goldmark.New(
		goldmark.WithExtensions(extensions...),
	)
}

func createDistFile(dist string) (*os.File, error) {
	dir := filepath.Dir(dist)
	if !existDir(dir) {
		os.MkdirAll(dir, os.ModePerm)
	}
	return os.Create(dist)
}
