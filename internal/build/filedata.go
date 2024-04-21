package build

// FileData is a struct for markdown file data.
type FileData struct {
	Path        string
	Content     string
	FrontMatter YamlFrontMatter
}
