'use strict';

import { MainDisplay, UnitDisplay } from "./display.js";
import { ExtSseq } from "./sseq.js";
import { renderLaTeX, download } from "./utils.js";

function send(msg) {
    window.webSocket.send(JSON.stringify(msg));
}

window.send = send;

function save() {
    let list = [];
    list = list.concat(mainSseq.history);
    let filename = prompt("Input filename");
    download(filename, list.map(JSON.stringify).join("\n"), "text/plain");
}
window.save = save;
window.mainSseq = new ExtSseq("Main", -96);
window.mainSseq.isUnit = false;
window.mainSseq.p = 2;
window.mainSseq.maxDegree = 96;

window.display = new MainDisplay("#main", mainSseq, false);
window.display.runningSign.style.display = "none";

window.webSocket = new WebSocket(`ws://${window.location.host}/ws`);

webSocket.onopen = function(e) {
    send({
        recipients: [],
        sseq : "Main",
        action : {
            "Construct": {
                algebra_name : "adem",
                module_name : "S_2",
            }
        }
    });
}
webSocket.onmessage = function(e) {
    let data = JSON.parse(e.data);
    try {
        let command = Object.keys(data.action)[0];
        window.mainSseq["process" + command](data.action[command]);
    } catch (err) {
        console.log("Unable to process message");
        console.log(data);
        console.log(`Error: ${err}`);
    }
}
