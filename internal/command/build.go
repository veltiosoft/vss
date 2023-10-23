package command

import "log"

type BuildCommand struct{}

func (c *BuildCommand) Help() string {
	return "Help text for foo"
}

func (c *BuildCommand) Run(args []string) int {
	log.Println("foo!")
	return 0
}

func (c *BuildCommand) Synopsis() string {
	return "Build site"
}
