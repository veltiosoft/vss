package build

type YamlFrontMatter struct {
	Author      string   `yaml:"author"`
	Title       string   `yaml:"title"`
	PubDatetime string   `yaml:"pubDatetime"`
	PostSlug    string   `yaml:"postSlug"`
	Description string   `yaml:"description"`
	Tags        []string `yaml:"tags"`
	Emoji       string   `yaml:"emoji"`
}

func (y *YamlFrontMatter) AsMap() map[string]interface{} {
	return map[string]interface{}{
		"author":      y.Author,
		"title":       y.Title,
		"pubDatetime": y.PubDatetime,
		"postSlug":    y.PostSlug,
		"description": y.Description,
		"tags":        y.Tags,
		"emoji":       y.Emoji,
	}
}
