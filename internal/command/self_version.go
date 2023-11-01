package command

import (
	"fmt"
	"runtime"
)

type SelfVersionCommand struct {
	Meta
}

func (c *SelfVersionCommand) Help() string {
	return "Usage: vss self version"
}

func (c *SelfVersionCommand) Synopsis() string {
	return "Show vss(self) version"
}

func (c *SelfVersionCommand) Run(args []string) int {
	fmt.Printf("vss %s\n", c.Version)
	fmt.Printf("commit %s\n", c.Revision)
	fmt.Printf("platform %s(%s)\n", runtime.GOOS, runtime.GOARCH)
	return 0
}
