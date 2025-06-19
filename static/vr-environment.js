// VR Environment Manager for Desktop Streamer
class VREnvironmentManager {
    constructor() {
        this.environments = new Map();
        this.currentEnvironment = null;
        this.renderer = null;
        this.scene = null;
        this.camera = null;
        this.vrDisplay = null;
        this.frameData = null;
        
        this.initializeEnvironments();
    }
    
    initializeEnvironments() {
        // Define VR environments
        this.environments.set('desktop', {
            name: 'Ambiente Desktop',
            description: 'Ambiente de escritório virtual',
            skybox: 'linear-gradient(180deg, #87CEEB 0%, #98FB98 100%)',
            lighting: 'bright',
            objects: ['desk', 'monitor', 'keyboard']
        });
        
        this.environments.set('cinema', {
            name: 'Cinema Virtual',
            description: 'Sala de cinema para visualização imersiva',
            skybox: 'linear-gradient(180deg, #000000 0%, #1a1a1a 100%)',
            lighting: 'dim',
            objects: ['screen', 'seats', 'projector']
        });
        
        this.environments.set('space', {
            name: 'Estação Espacial',
            description: 'Ambiente futurista no espaço',
            skybox: 'radial-gradient(circle, #000428 0%, #004e92 100%)',
            lighting: 'ambient',
            objects: ['hologram', 'console', 'stars']
        });
        
        this.environments.set('nature', {
            name: 'Natureza',
            description: 'Ambiente natural relaxante',
            skybox: 'linear-gradient(180deg, #87CEEB 0%, #228B22 100%)',
            lighting: 'natural',
            objects: ['trees', 'grass', 'mountains']
        });
    }
    
    async initializeVR() {
        try {
            // Check for WebXR support
            if ('xr' in navigator) {
                const isSupported = await navigator.xr.isSessionSupported('immersive-vr');
                if (isSupported) {
                    console.log('WebXR VR supported');
                    return true;
                }
            }
            
            // Fallback to WebVR
            if ('getVRDisplays' in navigator) {
                const displays = await navigator.getVRDisplays();
                if (displays.length > 0) {
                    this.vrDisplay = displays[0];
                    this.frameData = new VRFrameData();
                    console.log('WebVR supported');
                    return true;
                }
            }
            
            console.log('VR not supported, using fallback mode');
            return false;
        } catch (error) {
            console.error('Error initializing VR:', error);
            return false;
        }
    }
    
    createVREnvironment(environmentId, videoElement) {
        const environment = this.environments.get(environmentId);
        if (!environment) {
            throw new Error(`Environment ${environmentId} not found`);
        }
        
        // Create VR container
        const vrContainer = document.createElement('div');
        vrContainer.className = 'vr-environment';
        vrContainer.style.cssText = `
            position: fixed;
            top: 0;
            left: 0;
            width: 100vw;
            height: 100vh;
            background: ${environment.skybox};
            z-index: 10000;
            display: none;
            overflow: hidden;
        `;
        
        // Create 3D scene container
        const sceneContainer = document.createElement('div');
        sceneContainer.className = 'vr-scene';
        sceneContainer.style.cssText = `
            position: relative;
            width: 100%;
            height: 100%;
            perspective: 1000px;
            transform-style: preserve-3d;
        `;
        
        // Create video screen in 3D space
        const videoScreen = document.createElement('div');
        videoScreen.className = 'vr-video-screen';
        videoScreen.style.cssText = `
            position: absolute;
            top: 50%;
            left: 50%;
            width: 80vw;
            height: 45vw;
            max-width: 1200px;
            max-height: 675px;
            transform: translate(-50%, -50%) translateZ(0px);
            background: #000;
            border-radius: 20px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
            overflow: hidden;
            transition: transform 0.3s ease;
        `;
        
        // Clone video element for VR
        const vrVideo = videoElement.cloneNode(true);
        vrVideo.style.cssText = `
            width: 100%;
            height: 100%;
            object-fit: contain;
        `;
        vrVideo.srcObject = videoElement.srcObject;
        
        videoScreen.appendChild(vrVideo);
        
        // Add environment objects
        this.addEnvironmentObjects(sceneContainer, environment);
        
        sceneContainer.appendChild(videoScreen);
        vrContainer.appendChild(sceneContainer);
        
        // Add controls
        this.addVRControls(vrContainer, environmentId);
        
        document.body.appendChild(vrContainer);
        
        return {
            container: vrContainer,
            videoScreen: videoScreen,
            video: vrVideo,
            environment: environment
        };
    }
    
    addEnvironmentObjects(container, environment) {
        environment.objects.forEach((objectType, index) => {
            const object = document.createElement('div');
            object.className = `vr-object vr-${objectType}`;
            
            switch (objectType) {
                case 'desk':
                    object.style.cssText = `
                        position: absolute;
                        bottom: 10%;
                        left: 50%;
                        width: 200px;
                        height: 100px;
                        background: linear-gradient(45deg, #8B4513, #A0522D);
                        transform: translateX(-50%) rotateX(60deg);
                        border-radius: 10px;
                        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
                    `;
                    break;
                    
                case 'stars':
                    for (let i = 0; i < 50; i++) {
                        const star = document.createElement('div');
                        star.style.cssText = `
                            position: absolute;
                            width: 2px;
                            height: 2px;
                            background: white;
                            border-radius: 50%;
                            top: ${Math.random() * 100}%;
                            left: ${Math.random() * 100}%;
                            animation: twinkle ${2 + Math.random() * 3}s infinite;
                        `;
                        object.appendChild(star);
                    }
                    break;
                    
                case 'trees':
                    object.style.cssText = `
                        position: absolute;
                        bottom: 0;
                        left: ${20 + index * 15}%;
                        width: 60px;
                        height: 120px;
                        background: linear-gradient(180deg, #228B22, #006400);
                        clip-path: polygon(50% 0%, 0% 100%, 100% 100%);
                        transform: translateZ(-200px);
                    `;
                    break;
                    
                default:
                    object.style.cssText = `
                        position: absolute;
                        width: 50px;
                        height: 50px;
                        background: rgba(255, 255, 255, 0.1);
                        border-radius: 50%;
                        top: ${50 + Math.random() * 30}%;
                        left: ${20 + index * 20}%;
                        transform: translateZ(-100px);
                    `;
            }
            
            container.appendChild(object);
        });
    }
    
    addVRControls(container, environmentId) {
        const controls = document.createElement('div');
        controls.className = 'vr-controls';
        controls.style.cssText = `
            position: absolute;
            bottom: 20px;
            left: 50%;
            transform: translateX(-50%);
            display: flex;
            gap: 15px;
            z-index: 1000;
        `;
        
        // Environment selector
        const envSelector = document.createElement('select');
        envSelector.className = 'vr-env-selector';
        envSelector.style.cssText = `
            padding: 10px 15px;
            background: rgba(0, 0, 0, 0.8);
            color: white;
            border: 1px solid rgba(255, 255, 255, 0.3);
            border-radius: 25px;
            font-size: 14px;
            cursor: pointer;
        `;
        
        this.environments.forEach((env, id) => {
            const option = document.createElement('option');
            option.value = id;
            option.textContent = env.name;
            option.selected = id === environmentId;
            envSelector.appendChild(option);
        });
        
        envSelector.addEventListener('change', (e) => {
            this.switchEnvironment(e.target.value);
        });
        
        // Exit VR button
        const exitBtn = document.createElement('button');
        exitBtn.textContent = 'Sair do VR';
        exitBtn.style.cssText = `
            padding: 10px 20px;
            background: rgba(239, 68, 68, 0.9);
            color: white;
            border: none;
            border-radius: 25px;
            font-size: 14px;
            cursor: pointer;
            transition: all 0.3s ease;
        `;
        
        exitBtn.addEventListener('click', () => {
            this.exitVR();
        });
        
        exitBtn.addEventListener('mouseenter', () => {
            exitBtn.style.background = 'rgba(239, 68, 68, 1)';
            exitBtn.style.transform = 'scale(1.05)';
        });
        
        exitBtn.addEventListener('mouseleave', () => {
            exitBtn.style.background = 'rgba(239, 68, 68, 0.9)';
            exitBtn.style.transform = 'scale(1)';
        });
        
        controls.appendChild(envSelector);
        controls.appendChild(exitBtn);
        container.appendChild(controls);
    }
    
    async enterVR(environmentId = 'desktop', videoElement) {
        try {
            const vrSupported = await this.initializeVR();
            
            // Create VR environment
            const vrEnv = this.createVREnvironment(environmentId, videoElement);
            this.currentEnvironment = vrEnv;
            
            // Show VR environment
            vrEnv.container.style.display = 'block';
            
            // Add CSS animations
            this.addVRAnimations();
            
            // Start VR session if supported
            if (vrSupported && navigator.xr) {
                try {
                    const session = await navigator.xr.requestSession('immersive-vr');
                    this.handleXRSession(session);
                } catch (error) {
                    console.log('XR session failed, using fallback mode');
                }
            }
            
            // Add keyboard controls
            this.addKeyboardControls();
            
            console.log('VR mode activated');
            return true;
            
        } catch (error) {
            console.error('Error entering VR:', error);
            return false;
        }
    }
    
    addVRAnimations() {
        const style = document.createElement('style');
        style.textContent = `
            @keyframes twinkle {
                0%, 100% { opacity: 0.3; }
                50% { opacity: 1; }
            }
            
            @keyframes float {
                0%, 100% { transform: translateY(0px); }
                50% { transform: translateY(-10px); }
            }
            
            .vr-object {
                animation: float 4s ease-in-out infinite;
            }
            
            .vr-video-screen:hover {
                transform: translate(-50%, -50%) translateZ(50px) scale(1.02);
            }
            
            .vr-environment {
                animation: fadeIn 1s ease-in-out;
            }
            
            @keyframes fadeIn {
                from { opacity: 0; }
                to { opacity: 1; }
            }
        `;
        document.head.appendChild(style);
    }
    
    addKeyboardControls() {
        const handleKeyPress = (event) => {
            if (!this.currentEnvironment) return;
            
            const videoScreen = this.currentEnvironment.videoScreen;
            const currentTransform = videoScreen.style.transform;
            
            switch (event.key) {
                case 'ArrowUp':
                    event.preventDefault();
                    this.moveScreen(0, -50, 0);
                    break;
                case 'ArrowDown':
                    event.preventDefault();
                    this.moveScreen(0, 50, 0);
                    break;
                case 'ArrowLeft':
                    event.preventDefault();
                    this.moveScreen(-50, 0, 0);
                    break;
                case 'ArrowRight':
                    event.preventDefault();
                    this.moveScreen(50, 0, 0);
                    break;
                case '+':
                case '=':
                    event.preventDefault();
                    this.scaleScreen(1.1);
                    break;
                case '-':
                    event.preventDefault();
                    this.scaleScreen(0.9);
                    break;
                case 'r':
                case 'R':
                    event.preventDefault();
                    this.resetScreenPosition();
                    break;
                case 'Escape':
                    event.preventDefault();
                    this.exitVR();
                    break;
            }
        };
        
        document.addEventListener('keydown', handleKeyPress);
        this.keyboardHandler = handleKeyPress;
    }
    
    moveScreen(x, y, z) {
        if (!this.currentEnvironment) return;
        
        const screen = this.currentEnvironment.videoScreen;
        const currentTransform = screen.style.transform;
        
        // Parse current transform
        const translateMatch = currentTransform.match(/translate\(([^)]+)\)/);
        const translateZMatch = currentTransform.match(/translateZ\(([^)]+)\)/);
        
        let currentX = 0, currentY = 0, currentZ = 0;
        
        if (translateMatch) {
            const values = translateMatch[1].split(',');
            currentX = parseFloat(values[0]) || 0;
            currentY = parseFloat(values[1]) || 0;
        }
        
        if (translateZMatch) {
            currentZ = parseFloat(translateZMatch[1]) || 0;
        }
        
        const newX = currentX + x;
        const newY = currentY + y;
        const newZ = currentZ + z;
        
        screen.style.transform = `translate(${newX}px, ${newY}px) translateZ(${newZ}px)`;
    }
    
    scaleScreen(factor) {
        if (!this.currentEnvironment) return;
        
        const screen = this.currentEnvironment.videoScreen;
        const currentTransform = screen.style.transform;
        
        // Parse current scale
        const scaleMatch = currentTransform.match(/scale\(([^)]+)\)/);
        let currentScale = 1;
        
        if (scaleMatch) {
            currentScale = parseFloat(scaleMatch[1]) || 1;
        }
        
        const newScale = Math.max(0.5, Math.min(3, currentScale * factor));
        
        // Update transform with new scale
        let newTransform = currentTransform;
        if (scaleMatch) {
            newTransform = newTransform.replace(/scale\([^)]+\)/, `scale(${newScale})`);
        } else {
            newTransform += ` scale(${newScale})`;
        }
        
        screen.style.transform = newTransform;
    }
    
    resetScreenPosition() {
        if (!this.currentEnvironment) return;
        
        const screen = this.currentEnvironment.videoScreen;
        screen.style.transform = 'translate(-50%, -50%) translateZ(0px)';
    }
    
    switchEnvironment(environmentId) {
        if (!this.currentEnvironment) return;
        
        const environment = this.environments.get(environmentId);
        if (!environment) return;
        
        const container = this.currentEnvironment.container;
        const sceneContainer = container.querySelector('.vr-scene');
        
        // Update background
        container.style.background = environment.skybox;
        
        // Remove old objects
        const oldObjects = sceneContainer.querySelectorAll('.vr-object');
        oldObjects.forEach(obj => obj.remove());
        
        // Add new objects
        this.addEnvironmentObjects(sceneContainer, environment);
        
        console.log(`Switched to environment: ${environment.name}`);
    }
    
    handleXRSession(session) {
        // Handle WebXR session
        session.addEventListener('end', () => {
            console.log('XR session ended');
        });
        
        // Set up render loop for XR
        const onXRFrame = (time, frame) => {
            session.requestAnimationFrame(onXRFrame);
            
            // Update VR display
            if (frame) {
                // Handle XR frame rendering
                this.renderXRFrame(frame);
            }
        };
        
        session.requestAnimationFrame(onXRFrame);
    }
    
    renderXRFrame(frame) {
        // Implement XR frame rendering
        // This would typically involve WebGL rendering
        // For now, we'll use CSS 3D transforms as fallback
    }
    
    exitVR() {
        if (this.currentEnvironment) {
            this.currentEnvironment.container.remove();
            this.currentEnvironment = null;
        }
        
        if (this.keyboardHandler) {
            document.removeEventListener('keydown', this.keyboardHandler);
            this.keyboardHandler = null;
        }
        
        console.log('VR mode exited');
    }
    
    getAvailableEnvironments() {
        return Array.from(this.environments.entries()).map(([id, env]) => ({
            id,
            name: env.name,
            description: env.description
        }));
    }
}

// Export for use in main application
window.VREnvironmentManager = VREnvironmentManager;

