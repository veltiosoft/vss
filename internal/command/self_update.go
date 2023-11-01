package command

import (
	"archive/tar"
	"compress/gzip"
	"fmt"
	"io"
	"log"
	"net/http"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/minio/selfupdate"
)

const latestReleaseUrl = "https://github.com/vssio/go-vss/releases/latest/download"

type SelfUpdateCommand struct {
	Meta
}

func (c *SelfUpdateCommand) Help() string {
	return "Usage: vss self update"
}

func (c *SelfUpdateCommand) Synopsis() string {
	return "Update vss(self) command"
}

func (c *SelfUpdateCommand) Run(args []string) int {
	fmt.Println("Updating to latest ...")
	var ext string
	if runtime.GOOS == "linux" {
		ext = "tar.gz"
	} else {
		ext = "zip"
	}
	target := fmt.Sprintf("%s/vss_%s_%s.%s", latestReleaseUrl, runtime.GOOS, runtime.GOARCH, ext)
	err := downloadAndExtract(target)
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}
	// vss --version でバージョンが表示されるようにする
	// 出力は標準出力になる
	cmd := exec.Command("vss", "self", "version")
	output, err := cmd.CombinedOutput()
	if err != nil {
		// when unsupported self version command
		cmd := exec.Command("vss", "--version")
		output, err = cmd.CombinedOutput()
		if err != nil {
			log.Printf("[ERROR] %s", err)
			return 1
		}
	}
	fmt.Print(string(output))
	return 0
}

func updateBinary(target io.Reader) error {
	err := selfupdate.Apply(target, selfupdate.Options{})
	if err != nil {
		return err
	}
	return nil
}

func downloadAndExtract(url string) error {
	// Get the data
	resp, err := http.Get(url)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	// Create a new gzip reader
	gr, err := gzip.NewReader(resp.Body)
	if err != nil {
		return err
	}
	defer gr.Close()

	// Create a tar reader
	tr := tar.NewReader(gr)

	// Iterate through the files in the archive
	for {
		header, err := tr.Next()
		if err == io.EOF {
			break // End of archive
		}
		if err != nil {
			return err
		}

		// Open the output file
		if header.FileInfo().IsDir() {
			continue
		}
		filename := filepath.Join(strings.Split(header.Name, "/")[1:]...)
		if filename == "vss" {
			updateBinary(tr)
			break // end of update binary
		} else {
			continue
		}
	}

	return nil
}
