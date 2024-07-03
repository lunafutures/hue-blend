import { ObjectSchema } from "joi";

export class EnvValidator<T> {
	schema: ObjectSchema<T>;
	processEnv: T | undefined;

	constructor(schema: ObjectSchema<T>) {
		this.schema = schema;
	}

	initialize() {
		console.log("initializeing envvalidator", console.trace());
		const { error, value: processEnv } = this.schema.validate(
			process.env, { allowUnknown: true});
		if (error) {
			throw error;
		}

		this.processEnv = processEnv
	}

	getProperty<K extends keyof T>(property: K): T[K] {
		if (this.processEnv === undefined) {
			this.initialize()
		}

		if (this.processEnv === undefined || this.processEnv === null) {
			throw `processEnv still undefined or null despite validation.`;
		}

		return this.processEnv[property];
	}
}