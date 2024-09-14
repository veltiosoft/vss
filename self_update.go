package vss

import (
	"archive/tar"
	"archive/zip"
	"bytes"
	"compress/gzip"
	"errors"
	"fmt"
	"io"
	"log"
	"net/http"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	bufra "github.com/avvmoto/buf-readerat"
	"github.com/minio/selfupdate"
)

const (
	latestReleaseUrl = "https://github.com/vssio/go-vss/releases/latest/download"
	exe              = "vss"
)

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
	resp, err := http.Get(target)
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		log.Printf("[ERROR] %s", resp.Status)
		return 1
	}

	var r io.Reader
	if ext == "tar.gz" {
		r, err = extractTgz(resp)
		if err != nil {
			log.Printf("[ERROR] %s", err)
			return 1
		}
	} else {
		r, err = extractZip(resp)
		if err != nil {
			log.Printf("[ERROR] %s", err)
			return 1
		}
	}

	err = updateBinary(r)
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}
	log.Printf("[INFO] Successfully updated")

	cmd := exec.Command(exe, "self", "version")
	output, err := cmd.CombinedOutput()
	if err != nil {
		// when unsupported self version command
		cmd := exec.Command(exe, "--version")
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

// extractTgz extracts the binary from the tar.gz archive
func extractTgz(resp *http.Response) (io.Reader, error) {
	// Create a new gzip reader
	gr, err := gzip.NewReader(resp.Body)
	if err != nil {
		return nil, err
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
			return nil, err
		}

		// Open the output file
		if header.FileInfo().IsDir() {
			continue
		}
		filename := filepath.Join(strings.Split(header.Name, "/")[1:]...)
		if filename == binaryName {
			return tr, nil
		} else {
			continue
		}
	}

	return nil, errors.New("vss binary not found")
}

// extractZip extracts the binary from the zip archive
func extractZip(resp *http.Response) (io.Reader, error) {
	var buf bytes.Buffer
	_, err := io.Copy(&buf, resp.Body)
	if err != nil {
		return nil, err
	}
	if resp.ContentLength != int64(buf.Len()) {
		return nil, errors.New("content length mismatch")
	}

	bufr := bufra.NewBufReaderAt(bytes.NewReader(buf.Bytes()), buf.Len())
	r, err := zip.NewReader(bufr, int64(buf.Len()))
	if err != nil {
		return nil, err
	}

	for _, file := range r.File {
		filename := filepath.Join(strings.Split(file.Name, "/")[1:]...)
		if filename == binaryName {
			return file.Open()
		}
	}
	return nil, errors.New("vss binary not found")
}
