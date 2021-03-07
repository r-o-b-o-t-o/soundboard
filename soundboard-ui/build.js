const fsx = require("fs-extra");

async function build() {
	await fsx.copy("./static/", "dist/");
}

build();
