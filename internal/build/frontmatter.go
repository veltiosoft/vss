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
	PubDatetime string   `yaml:"pubDatetime"`
	PostSlug    string   `yaml:"postSlug"`
	Description string   `yaml:"description"`
	Tags        []string `yaml:"tags"`
	Emoji       string   `yaml:"emoji"`
	OgImage     string   `yaml:"ogImage"`
}

func (y *YamlFrontMatter) SaveTwemojiSvg(w io.Writer) error {
	r, _ := utf8.DecodeRuneInString(y.Emoji)

	// 小文字の16進数文字列に変換する
	codepoint := fmt.Sprintf("%04x", r)

	// svg 画像をダウンロードする
	res, err := http.Get(fmt.Sprintf("https://jdecked.github.io/twemoji/v/latest/svg/%s.svg", codepoint))
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
		"author":      y.Author,
		"title":       y.Title,
		"pubDatetime": y.PubDatetime,
		"postSlug":    y.PostSlug,
		"description": y.Description,
		"tags":        y.Tags,
		"emoji":       y.Emoji,
		"ogImage":     y.OgImage,
	}
}
