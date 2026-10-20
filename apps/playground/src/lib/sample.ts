export function sampleProgram() {
  return `import "intolang://builtin/std";

into message = "Hello World!";

init {
    println(message);
}
`;
}
