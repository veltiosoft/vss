package command

import (
	"log"

	"github.com/bradhe/stopwatch"
)

type BuildCommand struct {
	Meta
}

func (c *BuildCommand) Help() string {
	return "Help text for foo"
}

func (c *BuildCommand) Synopsis() string {
	return "Build site"
}

func (c *BuildCommand) Run(args []string) int {
	log.Println("[INFO] Build started")
	err := c.Meta.SetupConfig()
	if err != nil {
		log.Printf("[ERROR] loading config: %s", err)
		return 1
	}

	// init stop watch
	sw := stopwatch.Start()

	sw.Stop()
	log.Printf("[INFO] Build finished in %v", sw.Milliseconds())
	return 0
}
