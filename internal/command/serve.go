package command

import "log"

type ServeCommand struct {
	Meta
}

func (c *ServeCommand) Help() string {
	return "Help text for bar"
}

func (c *ServeCommand) Synopsis() string {
	return "Serve site"
}

func (c *ServeCommand) Run(args []string) int {
	log.Println("bar!")
	return 0
}
