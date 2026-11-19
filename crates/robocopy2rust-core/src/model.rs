use serde::{Deserialize, Serialize};

/// File data/metadata copy flags for `/COPY:<copyflags>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CopyFlags {
    /// D - Data
    pub data: bool,
    /// A - Attributes
    pub attributes: bool,
    /// T - Timestamps
    pub timestamps: bool,
    /// X - Skip alt data streams (ignored if /B or /ZB)
    pub skip_alt_data_streams: bool,
    /// S - Security (NTFS ACLs)
    pub security: bool,
    /// O - Owner info
    pub owner: bool,
    /// U - Auditing info
    pub auditing: bool,
}

impl Default for CopyFlags {
    fn default() -> Self {
        Self {
            data: true,
            attributes: true,
            timestamps: true,
            skip_alt_data_streams: false,
            security: false,
            owner: false,
            auditing: false,
        }
    }
}

impl CopyFlags {
    pub fn none() -> Self {
        Self {
            data: false,
            attributes: false,
            timestamps: false,
            skip_alt_data_streams: false,
            security: false,
            owner: false,
            auditing: false,
        }
    }

    pub fn copy_all() -> Self {
        Self {
            data: true,
            attributes: true,
            timestamps: true,
            skip_alt_data_streams: false,
            security: true,
            owner: true,
            auditing: true,
        }
    }

    pub fn sec() -> Self {
        Self {
            data: true,
            attributes: true,
            timestamps: true,
            skip_alt_data_streams: false,
            security: true,
            owner: false,
            auditing: false,
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn is_all(&self) -> bool {
        *self == Self::copy_all()
    }

    pub fn is_sec(&self) -> bool {
        *self == Self::sec()
    }

    pub fn is_empty(&self) -> bool {
        !self.data
            && !self.attributes
            && !self.timestamps
            && !self.skip_alt_data_streams
            && !self.security
            && !self.owner
            && !self.auditing
    }

    pub fn to_flag_string(&self) -> String {
        let mut s = String::new();
        if self.data {
            s.push('D');
        }
        if self.attributes {
            s.push('A');
        }
        if self.timestamps {
            s.push('T');
        }
        if self.security {
            s.push('S');
        }
        if self.owner {
            s.push('O');
        }
        if self.auditing {
            s.push('U');
        }
        if self.skip_alt_data_streams {
            s.push('X');
        }
        s
    }

    pub fn from_flag_string(raw: &str) -> Result<Self, String> {
        let mut flags = Self::none();
        for ch in raw.trim().chars() {
            match ch.to_ascii_uppercase() {
                'D' => flags.data = true,
                'A' => flags.attributes = true,
                'T' => flags.timestamps = true,
                'X' => flags.skip_alt_data_streams = true,
                'S' => flags.security = true,
                'O' => flags.owner = true,
                'U' => flags.auditing = true,
                other => return Err(format!("Unknown /COPY flag character '{other}'")),
            }
        }
        Ok(flags)
    }
}

/// Directory info copy flags for `/DCOPY:<copyflags>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DirCopyFlags {
    /// D - Data
    pub data: bool,
    /// A - Attributes
    pub attributes: bool,
    /// T - Timestamps
    pub timestamps: bool,
    /// E - Extended Attributes (not included in /COPYALL)
    pub extended_attributes: bool,
    /// X - Skip alt data streams
    pub skip_alt_data_streams: bool,
}

impl Default for DirCopyFlags {
    fn default() -> Self {
        Self {
            data: true,
            attributes: true,
            timestamps: false,
            extended_attributes: false,
            skip_alt_data_streams: false,
        }
    }
}

impl DirCopyFlags {
    pub fn none() -> Self {
        Self {
            data: false,
            attributes: false,
            timestamps: false,
            extended_attributes: false,
            skip_alt_data_streams: false,
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn is_empty(&self) -> bool {
        !self.data && !self.attributes && !self.timestamps && !self.extended_attributes && !self.skip_alt_data_streams
    }

    pub fn to_flag_string(&self) -> String {
        let mut s = String::new();
        if self.data {
            s.push('D');
        }
        if self.attributes {
            s.push('A');
        }
        if self.timestamps {
            s.push('T');
        }
        if self.extended_attributes {
            s.push('E');
        }
        if self.skip_alt_data_streams {
            s.push('X');
        }
        s
    }

    pub fn from_flag_string(raw: &str) -> Result<Self, String> {
        let mut flags = Self::none();
        for ch in raw.trim().chars() {
            match ch.to_ascii_uppercase() {
                'D' => flags.data = true,
                'A' => flags.attributes = true,
                'T' => flags.timestamps = true,
                'E' => flags.extended_attributes = true,
                'X' => flags.skip_alt_data_streams = true,
                other => return Err(format!("Unknown /DCOPY flag character '{other}'")),
            }
        }
        Ok(flags)
    }
}

/// NTFS/DOS file attributes `[RASHCNETO]` for `/A+`, `/A-`, `/IA`, and `/XA`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileAttrFlags {
    pub read_only: bool,   // R
    pub archive: bool,     // A
    pub system: bool,      // S
    pub hidden: bool,      // H
    pub compressed: bool,  // C
    pub not_indexed: bool, // N
    pub encrypted: bool,   // E
    pub temporary: bool,   // T
    pub offline: bool,     // O
}

impl FileAttrFlags {
    pub fn is_empty(&self) -> bool {
        !self.read_only
            && !self.archive
            && !self.system
            && !self.hidden
            && !self.compressed
            && !self.not_indexed
            && !self.encrypted
            && !self.temporary
            && !self.offline
    }

    pub fn to_flag_string(&self) -> String {
        let mut s = String::new();
        if self.read_only {
            s.push('R');
        }
        if self.archive {
            s.push('A');
        }
        if self.system {
            s.push('S');
        }
        if self.hidden {
            s.push('H');
        }
        if self.compressed {
            s.push('C');
        }
        if self.not_indexed {
            s.push('N');
        }
        if self.encrypted {
            s.push('E');
        }
        if self.temporary {
            s.push('T');
        }
        if self.offline {
            s.push('O');
        }
        s
    }

    pub fn from_flag_string(raw: &str) -> Result<Self, String> {
        let mut flags = Self::default();
        for ch in raw.trim().chars() {
            match ch.to_ascii_uppercase() {
                'R' => flags.read_only = true,
                'A' => flags.archive = true,
                'S' => flags.system = true,
                'H' => flags.hidden = true,
                'C' => flags.compressed = true,
                'N' => flags.not_indexed = true,
                'E' => flags.encrypted = true,
                'T' => flags.temporary = true,
                'O' => flags.offline = true,
                other => return Err(format!("Unknown attribute flag character '{other}'")),
            }
        }
        Ok(flags)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SparseMode {
    #[default]
    Default,
    Enable,  // /SPARSE:Y
    Disable, // /SPARSE:N
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CopyOptions {
    pub subdirectories_non_empty: bool,       // /S
    pub subdirectories_including_empty: bool, // /E
    pub max_levels: Option<u32>,              // /LEV:n
    pub restartable: bool,                    // /Z
    pub backup_mode: bool,                    // /B
    pub restartable_or_backup: bool,          // /ZB
    pub unbuffered_io: bool,                  // /J
    pub efs_raw: bool,                        // /EFSRAW
    pub copy_flags: CopyFlags,                // /COPY:
    pub no_copy: bool,                        // /NOCOPY
    pub sec_fix: bool,                        // /SECFIX
    pub tim_fix: bool,                        // /TIMFIX
    pub purge: bool,                          // /PURGE
    pub mirror: bool,                         // /MIR
    pub move_files: bool,                     // /MOV
    pub move_files_and_dirs: bool,            // /MOVE
    pub add_attributes: FileAttrFlags,        // /A+:
    pub remove_attributes: FileAttrFlags,     // /A-:
    pub create_tree_only: bool,               // /CREATE
    pub fat_names: bool,                      // /FAT
    pub disable_long_paths: bool,             // /256
    pub monitor_changes: Option<u32>,         // /MON:n
    pub monitor_minutes: Option<u32>,         // /MOT:m
    pub run_hours: Option<String>,            // /RH:hhmm-hhmm
    pub per_file_run_hours: bool,             // /PF
    pub inter_packet_gap_ms: Option<u32>,     // /IPG:n
    pub copy_junctions: bool,                 // /SJ
    pub copy_symlinks: bool,                  // /SL
    pub multithreaded: Option<u16>,           // /MT[:n] (1..=128)
    pub dir_copy_flags: DirCopyFlags,         // /DCOPY:
    pub no_dir_copy: bool,                    // /NODCOPY
    pub no_offload: bool,                     // /NOOFFLOAD
    pub network_compress: bool,               // /COMPRESS
    pub sparse_mode: SparseMode,              // /SPARSE[:Y/N]
    pub no_clone: bool,                       // /NOCLONE
}

impl Default for CopyOptions {
    fn default() -> Self {
        Self {
            subdirectories_non_empty: false,
            subdirectories_including_empty: true,
            max_levels: None,
            restartable: false,
            backup_mode: false,
            restartable_or_backup: false,
            unbuffered_io: false,
            efs_raw: false,
            copy_flags: CopyFlags::default(),
            no_copy: false,
            sec_fix: false,
            tim_fix: false,
            purge: false,
            mirror: false,
            move_files: false,
            move_files_and_dirs: false,
            add_attributes: FileAttrFlags::default(),
            remove_attributes: FileAttrFlags::default(),
            create_tree_only: false,
            fat_names: false,
            disable_long_paths: false,
            monitor_changes: None,
            monitor_minutes: None,
            run_hours: None,
            per_file_run_hours: false,
            inter_packet_gap_ms: None,
            copy_junctions: false,
            copy_symlinks: false,
            multithreaded: Some(8),
            dir_copy_flags: DirCopyFlags::default(),
            no_dir_copy: false,
            no_offload: false,
            network_compress: false,
            sparse_mode: SparseMode::Default,
            no_clone: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThrottleOptions {
    pub io_max_size: Option<String>, // /IoMaxSize:n[KMG]
    pub io_rate: Option<String>,     // /IoRate:n[KMG]
    pub threshold: Option<String>,   // /Threshold:n[KMG]
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileSelectionOptions {
    pub archive_only: bool,              // /A
    pub archive_and_reset: bool,         // /M
    pub include_attrs: FileAttrFlags,    // /IA:
    pub exclude_attrs: FileAttrFlags,    // /XA:
    pub exclude_files: Vec<String>,      // /XF
    pub exclude_dirs: Vec<String>,       // /XD
    pub exclude_changed: bool,           // /XC
    pub exclude_newer: bool,             // /XN
    pub exclude_older: bool,             // /XO
    pub exclude_extra: bool,             // /XX
    pub exclude_lonely: bool,            // /XL
    pub include_same: bool,              // /IS
    pub include_tweaked: bool,           // /IT
    pub include_modified: bool,          // /IM
    pub max_size_bytes: Option<u64>,     // /MAX:n
    pub min_size_bytes: Option<u64>,     // /MIN:n
    pub max_age: Option<String>,         // /MAXAGE:n
    pub min_age: Option<String>,         // /MINAGE:n
    pub max_last_access: Option<String>, // /MAXLAD:n
    pub min_last_access: Option<String>, // /MINLAD:n
    pub fat_file_times: bool,            // /FFT
    pub compensate_dst: bool,            // /DST
    pub exclude_junctions: bool,         // /XJ
    pub exclude_dir_junctions: bool,     // /XJD
    pub exclude_file_symlinks: bool,     // /XJF
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RetryOptions {
    pub retries: Option<u32>,                 // /R:n (sensible default: 3 vs Robocopy's 1M)
    pub wait_seconds: Option<u32>,            // /W:n (sensible default: 5 vs Robocopy's 30s)
    pub save_defaults_to_registry: bool,      // /REG
    pub wait_for_sharenames: bool,            // /TBD
    pub low_free_space_mode: bool,            // /LFSM
    pub low_free_space_floor: Option<String>, // /LFSM:n[KMG]
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self {
            retries: Some(3),
            wait_seconds: Some(5),
            save_defaults_to_registry: false,
            wait_for_sharenames: false,
            low_free_space_mode: false,
            low_free_space_floor: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFileMode {
    #[default]
    None,
    Overwrite,        // /LOG:
    Append,           // /LOG+:
    UnicodeOverwrite, // /UNILOG:
    UnicodeAppend,    // /UNILOG+:
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingOptions {
    pub list_only: bool,          // /L
    pub report_extra: bool,       // /X
    pub verbose: bool,            // /V
    pub include_timestamps: bool, // /TS
    pub full_paths: bool,         // /FP
    pub sizes_in_bytes: bool,     // /BYTES
    pub no_sizes: bool,           // /NS
    pub no_classes: bool,         // /NC
    pub no_file_list: bool,       // /NFL
    pub no_dir_list: bool,        // /NDL
    pub no_progress: bool,        // /NP
    pub show_eta: bool,           // /ETA
    pub log_mode: LogFileMode,    // /LOG, /LOG+, /UNILOG, /UNILOG+
    pub log_path: Option<String>, // file path for log_mode
    pub tee: bool,                // /TEE
    pub no_job_header: bool,      // /NJH
    pub no_job_summary: bool,     // /NJS
    pub unicode_output: bool,     // /UNICODE
}

impl Default for LoggingOptions {
    fn default() -> Self {
        Self {
            list_only: false,
            report_extra: false,
            verbose: false,
            include_timestamps: false,
            full_paths: false,
            sizes_in_bytes: false,
            no_sizes: false,
            no_classes: false,
            no_file_list: false,
            no_dir_list: false,
            no_progress: false,
            show_eta: true,
            log_mode: LogFileMode::None,
            log_path: None,
            tee: false,
            no_job_header: false,
            no_job_summary: false,
            unicode_output: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct JobFileOptions {
    pub load_job: Option<String>, // /JOB:jobname
    pub save_job: Option<String>, // /SAVE:jobname
    pub quit_after_parse: bool,   // /QUIT
    pub no_source_dir: bool,      // /NOSD
    pub no_dest_dir: bool,        // /NODD
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RobocopyJob {
    pub name: String,
    pub description: String,
    pub source: String,
    pub destination: String,
    pub file_specs: Vec<String>,
    pub copy: CopyOptions,
    pub selection: FileSelectionOptions,
    pub throttle: ThrottleOptions,
    pub retry: RetryOptions,
    pub logging: LoggingOptions,
    pub job_files: JobFileOptions,
    pub extra_args: Vec<String>,
}

impl Default for RobocopyJob {
    fn default() -> Self {
        Self {
            name: "Custom Robocopy Job".to_string(),
            description: "Incremental directory copy with safe retry defaults".to_string(),
            source: String::new(),
            destination: String::new(),
            file_specs: Vec::new(),
            copy: CopyOptions::default(),
            selection: FileSelectionOptions::default(),
            throttle: ThrottleOptions::default(),
            retry: RetryOptions::default(),
            logging: LoggingOptions::default(),
            job_files: JobFileOptions::default(),
            extra_args: Vec::new(),
        }
    }
}

impl RobocopyJob {
    /// Returns an unopinionated baseline matching Microsoft Robocopy's native CLI defaults
    /// (used by the command-line and `.rcj` parsers so only explicit switches appear).
    pub fn empty_cli_defaults() -> Self {
        Self {
            name: "Imported Robocopy Job".to_string(),
            description: String::new(),
            source: String::new(),
            destination: String::new(),
            file_specs: Vec::new(),
            copy: CopyOptions {
                subdirectories_including_empty: false,
                multithreaded: None,
                ..CopyOptions::default()
            },
            selection: FileSelectionOptions::default(),
            throttle: ThrottleOptions::default(),
            retry: RetryOptions {
                retries: None,
                wait_seconds: None,
                ..RetryOptions::default()
            },
            logging: LoggingOptions {
                show_eta: false,
                ..LoggingOptions::default()
            },
            job_files: JobFileOptions::default(),
            extra_args: Vec::new(),
        }
    }

    /// True if executing this job can delete or remove destination/source files (`/MIR`, `/PURGE`, `/MOVE`, `/MOV`)
    /// while not in `/L` list-only dry-run mode.
    pub fn has_destructive_flags(&self) -> bool {
        if self.logging.list_only {
            return false;
        }
        self.copy.mirror || self.copy.purge || self.copy.move_files || self.copy.move_files_and_dirs
    }

    /// Returns a list of active destructive switch names regardless of `/L`.
    pub fn destructive_switch_names(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.copy.mirror {
            out.push("/MIR");
        }
        if self.copy.purge {
            out.push("/PURGE");
        }
        if self.copy.move_files_and_dirs {
            out.push("/MOVE");
        } else if self.copy.move_files {
            out.push("/MOV");
        }
        out
    }

    /// Returns a safe `/L` list-only preview clone of this job with clear file-level output enabled.
    pub fn with_list_only(&self) -> Self {
        let mut cloned = self.clone();
        cloned.logging.list_only = true;
        cloned.logging.no_file_list = false;
        cloned.logging.full_paths = true;
        cloned.logging.sizes_in_bytes = true;
        cloned.logging.no_sizes = false;
        cloned.logging.no_classes = false;
        cloned
    }
}
