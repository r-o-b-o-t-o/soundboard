import changelog from "./changelog.js";
import { SocketWrapper } from "./socketWrapper.js";
import { Config, OutputDevice, Sound } from "./config.js";
import { doesNameMatchSearch, generateShortId, isVersionNewer, setTheme, soundNameSearchPreprocess } from "./utils.js";

$(() => {
	let currentVersion = "0.0.0";
	let config: Config = null;
	const socket = new SocketWrapper("ws://127.0.0.1:29816");
	const ignoreUpdates: boolean = JSON.parse(localStorage.getItem("ignoreUpdates") || "false");
	const requestCallbacks: { [key: number]: (data) => void } = {};

	socket
		.onOpen(() => {
			socket.send({ "message": "clientType", "clientType": "settings" });
			if (!ignoreUpdates) {
				socket.send({ "message": "isUpdateAvailable" });
			}
			socket.send({ "message": "outputDevices" });
			socket.send({ "message": "isAutostartEnabled" });
			socket.send({ "message": "currentVersion" });
		})
		.on("outputDevices", (data) => {
			loadDevices(data.devices);
			if (!config) {
				socket.send({ "message": "config" });
			}
		})
		.on("modifiedSound", (data) => editSound(data.previousPath, data.sound))
		.on("addedSounds", (data) => addSounds(data.sounds))
		.on("removedSound", (data) => removeSound(data.path))
		.on("config", (data) => {
			config = data.config;
			loadConfig();
		})
		.on("configOutputDevices", (data) => {
			config.outputDevices = data.config;
			loadConfigDevices();
		})
		.on("configSoundsConfig", (data) => {
			config.soundsConfig = data.config;
			loadConfigSoundsConfig();
		})
		.on("theme", (data) => setTheme(data.theme))
		.on("autostart", (data) => setAutostart(data.enabled))
		.on("updateAvailable", () => {
			if (!ignoreUpdates) {
				$("#modal-update-available").addClass("is-active");
			}
		})
		.on("currentVersion", (data) => {
			currentVersion = data.version;
			$("#version").text(`v${currentVersion}`);
		})
		.on("requestCallback", (data) => {
			const cb = requestCallbacks[data.requestId];
			if (cb) {
				delete requestCallbacks[data.requestId];
				cb(data);
			}
		})
		.on("error", (data) => console.error(data.error));

	function loadDevices(devices: string[]): void {
		$("#output-devices").empty();
		let i = 0;
		for (const dev of devices) {
			const $switch = $(
				`<div class="field">
					<input data-device="${dev}" id="output-device-${i}" type="checkbox" class="switch is-rounded is-info output-device">
					<label for="output-device-${i}">${dev}</label>
				</div>`
			);
			$switch.find(".output-device").on("change", () => {
				const devices: OutputDevice[] = $(".output-device")
					.filter((_idx, dev) => $(dev).prop("checked") === true)
					.map((_idx, dev) => {
						return {
							"name": $(dev).data("device"),
							"volume": 1.0,
						};
					})
					.toArray();
				socket.send({
					"message": "setOutputDevices",
					"devices": devices,
				});
			});
			$switch.appendTo("#output-devices");
			++i;
		}
	}

	function loadConfigDevices(): void {
		$(".output-device").prop("checked", false);
		for (const dev of config.outputDevices) {
			$(`.output-device[data-device="${dev.name}"]`).prop("checked", true);
		}
	}

	function loadConfigSoundsConfig(): void {
		$("#input-copy-sound").prop("checked", config.soundsConfig.copyFile);
	}

	function loadConfigSounds(): void {
		$("#sounds").empty();
		config.sounds = config.sounds.sort((a, b) => a.name.localeCompare(b.name));
		for (const sound of config.sounds) {
			const $sound = buildSound(sound);
			$sound.appendTo("#sounds");
		}
	}

	function loadConfig(): void {
		loadConfigDevices();
		loadConfigSoundsConfig();
		loadConfigSounds();
	}

	function setAutostart(enabled: boolean): void {
		$("#input-autostart").prop("checked", enabled);
	}

	function buildSound(sound: Sound): JQuery<HTMLElement> {
		const sendSound = () => {
			socket.send({
				"message": "editSound",
				"sound": sound,
			});
		};

		const $sound = $("#sound-template")
			.clone()
			.removeAttr("id")
			.attr("data-sound-path", sound.path);
		$sound.find(".sound-input-name").val(sound.name);
		$sound.find(".sound-input-path").val(sound.path);

		$sound.find(".sound-input-name").on("input", () => {
			sound.name = $sound.find(".sound-input-name").val() as string;
			sendSound();
		});

		$sound.find(".sound-input-path").on("input", () => {
			const prevPath = sound.path;
			sound.path = $sound.find(".sound-input-path").val() as string;
			$sound.attr("data-sound-path", sound.path);
			socket.send({
				"message": "editSound",
				"previousPath": prevPath,
				"sound": sound,
			});
		});

		$sound.find(".button-browse-sound").on("click", () => {
			const reqId = parseInt(generateShortId(), 36);
			requestCallbacks[reqId] = (data) => {
				const prevPath = sound.path;
				sound.path = data.newPath;
				socket.send({
					"message": "editSound",
					"previousPath": prevPath,
					"sound": sound,
					"receiveUpdate": true,
				});
			};
			socket.send({
				"message": "browseSoundDialog",
				"requestId": reqId,
			});
		});

		$sound.find(".sound-image-file, .sound-image-url, .sound-image-preview .sound-image-preview").hide();
		$sound.find("input[type=radio]").attr("name", `sound-input-image-type-${generateShortId()}`).parent().hide();
		$sound.find(".sound-input-image-type-file").on("change", () => {
			$sound.find(".sound-image-file").show();
			$sound.find(".sound-image-url").hide();
		});
		$sound.find(".sound-input-image-type-url").on("change", () => {
			$sound.find(".sound-image-url").show();
			$sound.find(".sound-image-file").hide();
		});
		$sound.find(".sound-checkbox-image").on("change", () => {
			const checked = $sound.find(".sound-checkbox-image").prop("checked");
			if (checked) {
				$sound.find("input[type=radio]").parent().show();
				$sound.find(".sound-input-image-type-file").prop("checked", "checked").trigger("change");
				$sound.find(".sound-image-preview").show();
			} else {
				sound.image = null;
				sendSound();
				$sound.find("input[type=radio]").parent().hide();
				$sound.find(".sound-image-file, .sound-image-url, .sound-image-preview").hide();
				$sound.find(".sound-image-preview").attr("src", "").hide();
				$sound.find(".sound-input-image-url, .sound-input-image-file").val("");
			}
		});
		$sound.find(".sound-input-image-url").on("input", () => {
			sound.image = { "url": $sound.find(".sound-input-image-url").val() as string };
			$sound.find(".sound-image-preview").attr("src", sound.image.url);
			sendSound();
		});
		$sound.find(".sound-input-image-file").on("input", () => {
			sound.image = { "file": $sound.find(".sound-input-image-file").val() as string };
			$sound.find(".sound-image-preview").attr("src", sound.image.file);
			sendSound();
		});
		if (sound.image) {
			$sound.find(".sound-checkbox-image").prop("checked", "checked");
			$sound.find(".sound-image-preview").show();
			$sound.find("input[type=radio]").parent().show();
			if (sound.image.file) {
				$sound.find(".sound-input-image-type-file")
					.prop("checked", "checked")
					.trigger("change");
				$sound.find(".sound-input-image-file").val(sound.image.file);
				$sound.find(".sound-image-preview").attr("src", sound.image.file);
			} else if (sound.image.url) {
				$sound.find(".sound-input-image-type-url")
					.prop("checked", "checked")
					.trigger("change");
				$sound.find(".sound-input-image-url").val(sound.image.url);
				$sound.find(".sound-image-preview").attr("src", sound.image.url);
			}
		}

		$sound.find(".sound-btn-delete").on("click", () => {
			$("#modal-delete-sound").find(".sound-name").text(sound.name);
			$("#modal-delete-sound").addClass("is-active");

			$("#modal-delete-sound .button-confirm").off("click").on("click", () => {
				$("#modal-delete-sound").removeClass("is-active");

				$sound.remove();

				socket.send({
					"message": "deleteSound",
					"sound": sound.path,
				});
			});
		});

		return $sound;
	}

	function editSound(prevPath: string, newSound: Sound): void {
		const $sounds = $(".sound");
		const sound = $sounds.toArray().find(s => $(s).data("sound-path") === prevPath);
		if (!sound) {
			return;
		}
		const $sound = $(sound);
		$sound.replaceWith(buildSound(newSound));
	}

	function addSounds(sounds: Sound[]): void {
		const search = getSearchFilter();

		for (const sound of sounds) {
			const $sound = buildSound(sound);
			if (!doesNameMatchSearch(sound.name, search)) {
				$sound.hide();
			}
			$sound.appendTo("#sounds");
		}
	}

	function removeSound(path: string): void {
		$("#sounds .sound").each((_idx, el) => {
			const $el = $(el);
			if ($el.data("sound-path") === path) {
				$el.remove();
			}
		});
	}

	function getSearchFilter(): string {
		let search: string = $("#settings-input-filter-sounds").val() as string;
		search = search.trim();
		if (search !== "") {
			search = soundNameSearchPreprocess(search);
		}
		return search;
	}

	function loadChangelog(): void {
		const latestChangelogVersionSeen = localStorage.getItem("latestChangelog");
		const versionsToDisplay: string[] = [];

		if (latestChangelogVersionSeen === null) {
			versionsToDisplay.push(Object.keys(changelog)[0]);
		} else {
			for (const version of Object.keys(changelog)) {
				if (isVersionNewer(latestChangelogVersionSeen, version)) {
					versionsToDisplay.push(version);
				}
			}
		}

		if (versionsToDisplay.length !== 0) {
			let display = false;
			for (const version of versionsToDisplay) {
				if (changelog[version].length === 0) {
					continue;
				}

				display = true;
				const $modalBody = $("#modal-changelog .modal-card-body");
				$modalBody.append(`<h4 class="is-title is-size-4">Version ${version}:</h4>`);
				const $ul = $("<ul>").appendTo($modalBody);
				for (const entry of changelog[version]) {
					let icon = "angle-right";
					if (entry.type === "addition") {
						icon = "plus";
					} else if (entry.type === "fix") {
						icon = "wrench";
					}

					$ul.append(`<li><i class="fa fa-${icon}"></i>&ensp;${entry.change}</li>`);
				}
			}

			if (display) {
				$("#modal-changelog").addClass("is-active");
			}
			localStorage.setItem("latestChangelog", versionsToDisplay[0]);
		}
	}

	$(".btn-add-sound").on("click", () => {
		socket.send({ "message": "addSoundsDialog" });
	});

	$("#settings-input-filter-sounds").on("input", () => {
		let search: string = $("#settings-input-filter-sounds").val() as string;
		const $sounds = $(".sound:not(#sound-template)").toArray().map(el => $(el));
		search = search.trim();

		for (const $sound of $sounds) {
			$sound.show();
		}

		if (search !== "") {
			search = soundNameSearchPreprocess(search);
			for (const $sound of $sounds) {
				let name: string = $sound.find(".sound-input-name").val() as string;
				const path: string = $sound.find(".sound-input-path").val() as string;
				name = soundNameSearchPreprocess(name);
				const found: boolean = name.indexOf(search) !== -1 || path.indexOf(search) !== -1;
				if (!found) {
					$sound.hide();
				}
			}
		}
	});

	$("#modal-update-available .button-update").one("click", () => {
		$("#modal-update-available .modal-card-body").html("Downloading update...");
		$("#modal-update-available .modal-card-foot").empty();
		socket.send({ "message": "performUpdate" });
	});

	$("#modal-update-available .button-never-ask").one("click", () => {
		localStorage.setItem("ignoreUpdates", JSON.stringify(true));
	});

	$("#input-autostart").on("change", () => {
		socket.send({ "message": "setAutostart", "enabled": $("#input-autostart").prop("checked") });
	});

	$("#input-copy-sound").on("change", () => {
		socket.send({
			"message": "setCopySound",
			"enabled": $("#input-copy-sound").prop("checked") as boolean,
		});
	});

	loadChangelog();
});
