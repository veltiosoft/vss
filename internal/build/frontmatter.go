package build

import (
	"fmt"
	"image"
	"image/png"
	"io"
	"net/http"
	"unicode/utf8"

	"github.com/srwiley/oksvg"
	"github.com/srwiley/rasterx"
)

const emojiPngSize = 400

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

	res, err := http.Get(fmt.Sprintf("https://jdecked.github.io/twemoji/v/latest/svg/%s.svg", codepoint))
	if err != nil {
		return err
	}
	if res.StatusCode != http.StatusOK {
		return fmt.Errorf("failed to download emoji: %s", res.Status)
	}

	// png の場合 svg から変換する
	if ext == "png" {
		icon, _ := oksvg.ReadIconStream(res.Body)
		icon.SetTarget(0, 0, float64(emojiPngSize), float64(emojiPngSize))
		rgba := image.NewRGBA(image.Rect(0, 0, emojiPngSize, emojiPngSize))
		icon.Draw(rasterx.NewDasher(emojiPngSize, emojiPngSize, rasterx.NewScannerGV(emojiPngSize, emojiPngSize, rgba, rgba.Bounds())), 1)
		return png.Encode(w, rgba)
	} else {
		_, err = io.Copy(w, res.Body)
		return err
	}
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
