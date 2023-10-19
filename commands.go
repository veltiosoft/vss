package vss

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
	return "Prints foo"
}

type ServeCommand struct{}

func (c *ServeCommand) Help() string {
	return "Help text for bar"
}

func (c *ServeCommand) Run(args []string) int {
	log.Println("bar!")
	return 0
}

func (c *ServeCommand) Synopsis() string {
	return "Prints bar"
}
