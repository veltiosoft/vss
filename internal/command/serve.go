package command

import (
	"log"
	"net/http"
	"os"
	"path/filepath"
	"slices"

	"github.com/fsnotify/fsnotify"
	"github.com/vssio/go-vss/internal/build"
	"github.com/vssio/go-vss/internal/config"
)

const port = "8080"

type htmlDir struct {
	dir http.Dir
}

func (d htmlDir) Open(name string) (http.File, error) {
	f, err := d.dir.Open(name + ".html")
	if os.IsNotExist(err) {
		if f, err := d.dir.Open(name); err == nil {
			return f, nil
		}
	}
	return f, err
}

type ServeCommand struct {
	Meta
	builder *build.Builder
}

func (c *ServeCommand) Help() string {
	return "Help text for bar"
}

func (c *ServeCommand) Synopsis() string {
	return "Serve site"
}

func (c *ServeCommand) Run(args []string) int {
	log.Printf("[INFO] serve started")

	config, err := config.LoadConfig()
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}
	// for local server
	config.BaseUrl = "http://localhost:" + port

	// init site
	c.builder = build.NewBuilder(config)
	err = c.builder.Run()
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}

	// watch for changes
	go c.watch()

	fs := http.FileServer(htmlDir{http.Dir(config.Dist)})
	http.Handle("/", http.StripPrefix("/", fs))

	log.Printf("[INFO] serving on http://localhost:%s\n", port)
	err = http.ListenAndServe(":"+port, nil)
	if err != nil {
		log.Printf("[ERROR] serve: %s", err)
		return 1
	}
	return 0
}

func (c *ServeCommand) watch() error {
	// dist/ 以下のディレクトリを無視するために、正規表現を生成する
	ignores := []string{
		c.builder.GetDistPath(),
		filepath.Join(c.builder.GetDistPath(), "*"),
	}
	dirs, err := getDirPathsRecursive(".", ignores)
	if err != nil {
		return err
	}
	// init watcher
	watcher, err := fsnotify.NewWatcher()
	if err != nil {
		return err
	}
	defer watcher.Close()

	for _, dir := range dirs {
		err = watcher.Add(dir)
		if err != nil {
			return err
		}
	}

	for {
		select {
		case event, ok := <-watcher.Events:
			if !ok {
				return nil
			}
			if event.Has(fsnotify.Write) {
				if slices.Contains(ignores, event.Name) {
					continue
				}
				log.Println("[INFO] modified file:", event.Name)
				c.builder.ReloadConfig()
				c.builder.SetBaseUrl("http://localhost:" + port)
				err = c.builder.Run()
				if err != nil {
					log.Printf("[ERROR] %s", err)
					return err
				}
			}
		case err, ok := <-watcher.Errors:
			if !ok {
				return nil
			}
			log.Println("error:", err)
		}
	}
}

// getDirPathsRecursive returns a slice of all directories in a given path.
func getDirPathsRecursive(path string, ignores []string) ([]string, error) {
	var filePaths []string

	err := filepath.Walk(path, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		// ファイルを無視する
		if !info.IsDir() {
			return nil
		}

		// ignore と正規表現で一致する場合は無視する
		for _, ignore := range ignores {
			if ignore != "" {
				matched, err := filepath.Match(ignore, path)
				if err != nil {
					return err
				}
				if matched {
					return nil
				}
			}
		}

		filePaths = append(filePaths, path)
		return nil
	})

	if err != nil {
		return nil, err
	}

	return filePaths, nil
}
