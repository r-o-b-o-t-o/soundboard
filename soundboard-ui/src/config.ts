export type SoundImage = {
	url?: string;
	file?: string;
};

export type Sound = {
	name: string;
	path: string;
	volume: number;
	image?: SoundImage;
};

export type OutputDevice = {
	name: string;
	volume: number;
};

export type SoundsConfig = {
	copyFile: boolean;
};

export type Config = {
	globalVolume: number;
	outputDevices: OutputDevice[];
	soundsConfig: SoundsConfig;
	sounds: Sound[];
};
