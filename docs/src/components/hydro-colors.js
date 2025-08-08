// Hydro Theme Color Palette
// Based on the primary Hydro colors: #0096FF to #0FDBA2

export const HydroColors = {
    // Primary brand colors
    primary: '#0096FF',
    secondary: '#0FDBA2',

    // Extended palette - vibrant colors that complement the Hydro theme
    blue: '#0096FF',
    teal: '#0FDBA2',
    purple: '#8B5CF6',
    pink: '#EC4899',
    orange: '#F97316',
    green: '#10B981',
    coral: '#FF6B6B',
    violet: '#A855F7',

    // Key group colors - diverse vibrant palette ordered for maximum contrast between adjacent colors
    keyColors: [
        '#10B981', // Green
        '#A855F7', // Violet
        '#E57373', // Soft red
        '#06B6D4', // Cyan
        '#8B5CF6', // Purple
        '#EC4899', // Pink
        '#FF6B6B', // Coral
        '#6366F1', // Indigo
        '#8B5A2B', // Brown
    ],

    // Gradients - diverse color combinations
    gradients: {
        primary: 'linear-gradient(135deg, #0096FF 0%, #0FDBA2 100%)',
        primaryHorizontal: 'linear-gradient(90deg, #0096FF 0%, #0FDBA2 100%)',
        sunset: 'linear-gradient(135deg, #F97316 0%, #EC4899 100%)',
        ocean: 'linear-gradient(135deg, #0096FF 0%, #06B6D4 100%)',
        forest: 'linear-gradient(135deg, #10B981 0%, #0FDBA2 100%)',
        cosmic: 'linear-gradient(135deg, #8B5CF6 0%, #A855F7 100%)',
        fire: 'linear-gradient(135deg, #E57373 0%, #F97316 100%)',
        coral: 'linear-gradient(135deg, #FF6B6B 0%, #EC4899 100%)',
        electric: 'linear-gradient(135deg, #6366F1 0%, #8B5CF6 100%)',
    },

    // Utility function to get a key color by index
    getKeyColor: (index) => {
        return HydroColors.keyColors[index % HydroColors.keyColors.length];
    },

    // Utility function to get a key color by key name (consistent hashing)
    getKeyColorByName: (keyName) => {
        // Simple hash function for consistent color assignment
        let hash = 0;
        for (let i = 0; i < keyName.length; i++) {
            const char = keyName.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash; // Convert to 32-bit integer
        }
        const index = Math.abs(hash) % HydroColors.keyColors.length;
        return HydroColors.keyColors[index];
    }
};

export default HydroColors;
