package command

import (
	"fmt"
	"log"

	"github.com/vssio/go-vss/internal/skeleton"
)

type NewCommand struct {
	Meta
}

func (c *NewCommand) Help() string {
	return `Usage: vss new <path>`
}

func (c *NewCommand) Synopsis() string {
	return "Generate a skeleton site"
}

func (c *NewCommand) Run(args []string) int {
	if len(args) != 1 {
		log.Printf("[ERROR] new command takes only one argument")
		fmt.Println(c.Help())
		return 1
	}
	log.Println("[INFO] generate skeleton started")
	err := skeleton.GenerateSkeleton(args[0])
	if err != nil {
		log.Printf("[ERROR] %s", err)
		return 1
	}
	log.Printf("[INFO] generate skeleton finished")
	return 0
}
