export interface TreeNode {
  label: string;
  children?: TreeNode[];
  ignored?: boolean;
  cut?: boolean;
  /** Full filesystem path (file explorer nodes). */
  path?: string;
  /** Directory flag (file explorer nodes). */
  is_dir?: boolean;
  /** True once a directory's children have been loaded. */
  loaded?: boolean;
  loading?: boolean;
}
