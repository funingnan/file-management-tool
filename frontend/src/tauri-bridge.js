// Tauri invoke bridge - replaces Wails bindings
const { invoke } = window.__TAURI__.core;

window.go = {
    main: {
        App: {
            GetAppMode: () => invoke('get_app_mode'),
            SetTagPickerMode: (filePath) => invoke('set_tag_picker_mode', { filePath }),
            CloseTagPicker: () => invoke('close_tag_picker'),
            GetSettings: () => invoke('get_settings'),
            SaveSettings: (s) => invoke('save_settings', { newSettings: s }),
            SelectFolder: () => invoke('select_folder'),
            ScanFolder: (path, types) => invoke('scan_folder', { folderPath: path, enabledTypes: types }),
            ListDocuments: (tagIds, search, untagged, types) => invoke('list_documents', { tagIds, searchText: search, untagged, fileTypes: types }),
            GetDocument: (id) => invoke('get_document', { id }),
            GetDocumentCount: () => invoke('get_document_count'),
            GetUntaggedCount: (types) => invoke('get_untagged_count', { fileTypes: types }),
            RemoveDocument: (id) => invoke('remove_document', { docId: id }),
            RemoveDocuments: (ids) => invoke('remove_documents', { docIds: ids }),
            GetFileTypeCounts: () => invoke('get_file_type_counts'),
            GetSupportedTypes: () => invoke('get_supported_types'),
            ListTags: () => invoke('list_tags'),
            AddTagToDocument: (id, name) => invoke('add_tag_to_document', { docId: id, tagName: name }),
            RemoveTagFromDocument: (docId, tagId) => invoke('remove_tag_from_document', { docId, tagId }),
            BatchAddTag: (ids, name) => invoke('batch_add_tag', { docIds: ids, tagName: name }),
            BatchRemoveTagFromDocuments: (ids, tagId) => invoke('batch_remove_tag_from_documents', { docIds: ids, tagId }),
            DeleteTag: (id) => invoke('delete_tag', { tagId: id }),
            RenameTag: (id, name) => invoke('rename_tag', { tagId: id, newName: name }),
            GetDocumentTagsByPath: (path) => invoke('get_document_tags_by_path', { filePath: path }),
            AddTagToFilePath: (path, name) => invoke('add_tag_to_file_path', { filePath: path, tagName: name }),
            RemoveTagFromFilePath: (path, tagId) => invoke('remove_tag_from_file_path', { filePath: path, tagId }),
            OpenFile: (id) => invoke('open_file', { docId: id }),
            OpenFileLocation: (id) => invoke('open_file_location', { docId: id }),
            GetDocumentGraph: () => invoke('get_document_graph'),
            GetTagGraph: () => invoke('get_tag_graph'),
            ExportDatabase: () => invoke('export_database'),
            ImportDatabase: () => invoke('import_database'),
        }
    }
};
