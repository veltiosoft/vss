package vss

import (
	"fmt"
	"testing"

	"github.com/cbroglie/mustache"
)

func TestConvertMarkdownPathToHtmlPath(t *testing.T) {
	tests := []struct {
		name         string
		markdownPath string
		expected     string
	}{
		{
			name:         "converts .md to .html",
			markdownPath: "example.md",
			expected:     "example.html",
		},
		{
			name:         "handles path contains a directory",
			markdownPath: "docs/tutorial.md",
			expected:     "docs/tutorial.html",
		},
		{
			name:         "handles multiple .md extensions",
			markdownPath: "docs/tutorial.md.md",
			expected:     "docs/tutorial.md.html",
		},
		{
			name:         "handles uppercase .MD extension",
			markdownPath: "README.MD",
			expected:     "README.html",
		},
		{
			name:         "no handles .markdown extension",
			markdownPath: "readme.markdown",
			expected:     "readme.markdown",
		},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			result := convertMarkdownPathToHtmlPath(test.markdownPath)
			if result != test.expected {
				t.Errorf("expected %s, but got %s", test.expected, result)
			}
		})
	}
}

func BenchmarkTemplateMapWithoutSize(b *testing.B) {
	builder := new(Builder)
	m := make(map[string]*mustache.Template)
	for i := 0; i < b.N; i++ {
		t, _ := mustache.ParseFile("testdata/test_template.html")
		m[fmt.Sprintf("%d_test.html", i)] = t
	}
	builder.templateMap = m
}

func BenchmarkTemplateMapWithSize(b *testing.B) {
	builder := new(Builder)
	m := make(map[string]*mustache.Template, b.N)
	for i := 0; i < b.N; i++ {
		t, _ := mustache.ParseFile("testdata/test_template.html")
		m[fmt.Sprintf("%d_test.html", i)] = t
	}
	builder.templateMap = m
}
