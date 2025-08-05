/**
 * Generate node colors dynamically based on provided node type configuration
 * @param {string} nodeType - The node type to get colors for
 * @param {string} paletteKey - The color palette to use
 * @param {Object} nodeTypeConfig - Configuration object with node type mappings
 * @returns {Object} Color configuration for the node type
 */
export function generateNodeColors(nodeType: string, paletteKey?: string, nodeTypeConfig?: any): any;
/**
 * Truncates a container name if it's longer than the specified max length
 * @param {string} name - The container name to truncate
 * @param {number} maxLength - Maximum length before truncation (default: 15)
 * @param {Object} options - Truncation options
 * @param {string} options.side - 'left' or 'right' truncation (default: 'left')
 * @param {boolean} options.splitOnDelimiter - Whether to favor splitting at delimiters (default: false)
 * @param {number} options.delimiterPenalty - Percentage penalty for delimiter split being longer (default: 0.2)
 * @returns {string} The truncated name with ellipsis if needed
 */
export function truncateContainerName(name: string, maxLength?: number, options?: {
    side: string;
    splitOnDelimiter: boolean;
    delimiterPenalty: number;
}): string;
//# sourceMappingURL=utils.d.ts.map