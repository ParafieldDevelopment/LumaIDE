export interface TreeNode {
  label: string;
  children?: TreeNode[];
  ignored?: boolean;
  cut?: boolean;
}
