#!/bin/env node

let obj = { tt_flows: [], avb_flows: [] };

function rand_max(x) {
    return Math.floor(Math.random() * x);
}
function rand_in(arr) {
    let i = rand_max(arr.length);
    return arr[i];
}

for(let i = 0; i < 10; i++) {
    let src = rand_max(10);
    let dst = rand_max(10);
    if(src == dst) {
        i--;
        continue;
    }
    let period = rand_in([100, 200, 250]);
    let tt_flow = {
        src,
        dst,
        size: rand_in([4500, 4500, 1500, 1500, 1500, 45000, 45000]),
        period,
        max_delay: period,
        offset: 0
    };
    obj.tt_flows.push(tt_flow);
}
for(let i = 0; i < 15; i++) {
    let src = rand_max(10);
    let dst = rand_max(10);
    if(src == dst) {
        i--;
        continue;
    }
    let period = rand_in([125, 200, 250]);
    let avb_flow = {
        src,
        dst,
        size: 400,
        period,
        max_delay: period,
        avb_type: rand_in(["A", "B"])
    };
    obj.avb_flows.push(avb_flow);
}

console.log(JSON.stringify(obj, undefined, 2));