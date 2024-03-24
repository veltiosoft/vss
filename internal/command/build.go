package command

import (
	"log"

	"github.com/bradhe/stopwatch"
	"github.com/vssio/go-vss/internal/build"
	"github.com/vssio/go-vss/internal/config"
)

type BuildCommand struct {
	Meta
}

func (c *BuildCommand) Help() string {
	return "Help text for foo"
}

func (c *BuildCommand) Synopsis() string {
	return "Build your vss site"
}

func (c *BuildCommand) Run(args []string) int {
	log.Println("[INFO] build started")
	// init stop watch
	sw := stopwatch.Start()

	config, err := config.LoadConfig()
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}

	builder := build.NewBuilder(config)
	err = builder.Run()
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}

	sw.Stop()
	log.Printf("[INFO] build finished in %v", sw.Milliseconds())
	return 0
}
