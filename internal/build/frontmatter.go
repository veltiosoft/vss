package build

import (
	"fmt"
	"io"
	"net/http"
	"unicode/utf8"
)

type YamlFrontMatter struct {
	Author      string   `yaml:"author"`
	Title       string   `yaml:"title"`
	PubDatetime string   `yaml:"pub_datetime"`
	PostSlug    string   `yaml:"post_slug"`
	Description string   `yaml:"description"`
	Tags        []string `yaml:"tags"`
	Emoji       string   `yaml:"emoji"`
	OgImage     string   `yaml:"og_image"`
}

func (y *YamlFrontMatter) SaveTwemojiSvg(w io.Writer) error {
	return y.saveTwemojiImage(w, "svg")
}

func (y *YamlFrontMatter) SaveTwemojiPng(w io.Writer) error {
	return y.saveTwemojiImage(w, "png")
}

func (y *YamlFrontMatter) saveTwemojiImage(w io.Writer, ext string) error {
	r, _ := utf8.DecodeRuneInString(y.Emoji)

	// 小文字の16進数文字列に変換する
	codepoint := fmt.Sprintf("%04x", r)

	if ext != ".svg" && ext != "png" {
		return fmt.Errorf("unsupported file extension: %s", ext)
	}
	var url string
	if ext == "svg" {
		url = fmt.Sprintf("https://jdecked.github.io/twemoji/v/latest/svg/%s.svg", codepoint)
	} else if ext == "png" {
		url = fmt.Sprintf("https://jdecked.github.io/twemoji/v/latest/72x72/%s.png", codepoint)
	}

	res, err := http.Get(url)
	if err != nil {
		return err
	}
	if res.StatusCode != http.StatusOK {
		return fmt.Errorf("failed to download emoji: %s", res.Status)
	}
	_, err = io.Copy(w, res.Body)
	return err
}

func (y *YamlFrontMatter) AsMap() map[string]interface{} {
	return map[string]interface{}{
		"author":       y.Author,
		"title":        y.Title,
		"pub_datetime": y.PubDatetime,
		"post_slug":    y.PostSlug,
		"description":  y.Description,
		"tags":         y.Tags,
		"emoji":        y.Emoji,
		"og_image":     y.OgImage,
	}
}
