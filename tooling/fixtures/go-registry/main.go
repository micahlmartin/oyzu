package main

import (
	"fmt"
	"github.com/google/uuid"
)

func identifier() string { return uuid.Nil.String() }
func main()              { fmt.Println(identifier()) }
