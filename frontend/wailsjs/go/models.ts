export namespace database {
	
	export class Document {
	    id: number;
	    path: string;
	    filename: string;
	    title: string;
	    file_type: string;
	    file_size: number;
	    // Go type: time
	    mod_time: any;
	    // Go type: time
	    created_at: any;
	    // Go type: time
	    indexed_at: any;
	
	    static createFrom(source: any = {}) {
	        return new Document(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.path = source["path"];
	        this.filename = source["filename"];
	        this.title = source["title"];
	        this.file_type = source["file_type"];
	        this.file_size = source["file_size"];
	        this.mod_time = this.convertValues(source["mod_time"], null);
	        this.created_at = this.convertValues(source["created_at"], null);
	        this.indexed_at = this.convertValues(source["indexed_at"], null);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class Tag {
	    id: number;
	    name: string;
	
	    static createFrom(source: any = {}) {
	        return new Tag(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	    }
	}
	export class DocumentDetail {
	    id: number;
	    path: string;
	    filename: string;
	    title: string;
	    file_type: string;
	    file_size: number;
	    // Go type: time
	    mod_time: any;
	    // Go type: time
	    created_at: any;
	    // Go type: time
	    indexed_at: any;
	    tags: Tag[];
	
	    static createFrom(source: any = {}) {
	        return new DocumentDetail(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.path = source["path"];
	        this.filename = source["filename"];
	        this.title = source["title"];
	        this.file_type = source["file_type"];
	        this.file_size = source["file_size"];
	        this.mod_time = this.convertValues(source["mod_time"], null);
	        this.created_at = this.convertValues(source["created_at"], null);
	        this.indexed_at = this.convertValues(source["indexed_at"], null);
	        this.tags = this.convertValues(source["tags"], Tag);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class GraphEdge {
	    from: number;
	    to: number;
	    weight: number;
	
	    static createFrom(source: any = {}) {
	        return new GraphEdge(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.from = source["from"];
	        this.to = source["to"];
	        this.weight = source["weight"];
	    }
	}
	export class GraphNode {
	    id: number;
	    label: string;
	    size: number;
	    group: number;
	
	    static createFrom(source: any = {}) {
	        return new GraphNode(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.label = source["label"];
	        this.size = source["size"];
	        this.group = source["group"];
	    }
	}
	export class GraphData {
	    nodes: GraphNode[];
	    edges: GraphEdge[];
	
	    static createFrom(source: any = {}) {
	        return new GraphData(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.nodes = this.convertValues(source["nodes"], GraphNode);
	        this.edges = this.convertValues(source["edges"], GraphEdge);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	
	
	
	export class TagWithCount {
	    id: number;
	    name: string;
	    count: number;
	
	    static createFrom(source: any = {}) {
	        return new TagWithCount(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	        this.count = source["count"];
	    }
	}

}

export namespace main {
	
	export class ScanFolderResult {
	    newFiles: number;
	    total: number;
	    relocated: number;
	
	    static createFrom(source: any = {}) {
	        return new ScanFolderResult(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.newFiles = source["newFiles"];
	        this.total = source["total"];
	        this.relocated = source["relocated"];
	    }
	}
	export class Settings {
	    enabledTypes: string[];
	    currentFolderPath: string;
	
	    static createFrom(source: any = {}) {
	        return new Settings(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.enabledTypes = source["enabledTypes"];
	        this.currentFolderPath = source["currentFolderPath"];
	    }
	}

}

