import init, { greet, test_hydroflow } from "wasm_test_site";
let { memory } = await init();

(window as any).writeToDom = function(str: string) {
    document.body.appendChild(document.createTextNode(str));
    document.body.append(document.createElement('br'));
};

greet();
test_hydroflow();
