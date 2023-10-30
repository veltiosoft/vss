package command

import (
	"log"

	"github.com/bradhe/stopwatch"
	"github.com/vssio/go-vss/internal/build"
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
	err := c.Meta.SetupConfig()
	if err != nil {
		log.Printf("[ERROR] loading config: %s", err)
		return 1
	}

	// init stop watch
	sw := stopwatch.Start()

	// TODO: build site
	builder := build.NewBuilder(c.Meta.Config)
	err = builder.Run()
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}
	sw.Stop()
	log.Printf("[INFO] build finished in %v", sw.Milliseconds())
	return 0
}
