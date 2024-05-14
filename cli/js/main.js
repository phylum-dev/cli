import * as Api from "ext:phylum_api/api.js";
import { ApiVersion } from "ext:phylum_api/api_version.js";

globalThis.Phylum ??= {};
Object.assign(globalThis.Phylum, Api);
globalThis.Phylum.ApiVersion = ApiVersion;
