// Performance Monitor for Desktop Streamer
class PerformanceMonitor {
    constructor() {
        this.metrics = {
            fps: 0,
            bitrate: 0,
            latency: 0,
            cpuUsage: 0,
            memoryUsage: 0,
            networkUsage: 0,
            droppedFrames: 0,
            jitter: 0
        };
        
        this.history = {
            fps: [],
            bitrate: [],
            latency: [],
            cpuUsage: [],
            memoryUsage: []
        };
        
        this.thresholds = {
            fps: { warning: 25, critical: 15 },
            latency: { warning: 100, critical: 200 },
            cpuUsage: { warning: 70, critical: 90 },
            memoryUsage: { warning: 80, critical: 95 },
            droppedFrames: { warning: 5, critical: 10 }
        };
        
        this.callbacks = new Map();
        this.isMonitoring = false;
        this.monitoringInterval = null;
        
        this.initializePerformanceAPI();
    }
    
    initializePerformanceAPI() {
        // Check for Performance Observer support
        if ('PerformanceObserver' in window) {
            this.setupPerformanceObserver();
        }
        
        // Check for Memory API support
        if ('memory' in performance) {
            this.memoryAPISupported = true;
        }
        
        // Setup network monitoring
        if ('connection' in navigator) {
            this.networkAPISupported = true;
            this.setupNetworkMonitoring();
        }
    }
    
    setupPerformanceObserver() {
        try {
            const observer = new PerformanceObserver((list) => {
                const entries = list.getEntries();
                entries.forEach(entry => {
                    if (entry.entryType === 'measure') {
                        this.processMeasurement(entry);
                    }
                });
            });
            
            observer.observe({ entryTypes: ['measure', 'navigation'] });
            this.performanceObserver = observer;
        } catch (error) {
            console.warn('Performance Observer not supported:', error);
        }
    }
    
    setupNetworkMonitoring() {
        const connection = navigator.connection;
        
        const updateNetworkInfo = () => {
            this.metrics.networkUsage = {
                effectiveType: connection.effectiveType,
                downlink: connection.downlink,
                rtt: connection.rtt,
                saveData: connection.saveData
            };
        };
        
        connection.addEventListener('change', updateNetworkInfo);
        updateNetworkInfo();
    }
    
    startMonitoring(interval = 1000) {
        if (this.isMonitoring) {
            this.stopMonitoring();
        }
        
        this.isMonitoring = true;
        this.monitoringInterval = setInterval(() => {
            this.collectMetrics();
            this.analyzePerformance();
            this.notifyCallbacks();
        }, interval);
        
        console.log('Performance monitoring started');
    }
    
    stopMonitoring() {
        if (this.monitoringInterval) {
            clearInterval(this.monitoringInterval);
            this.monitoringInterval = null;
        }
        
        this.isMonitoring = false;
        console.log('Performance monitoring stopped');
    }
    
    collectMetrics() {
        // Collect FPS
        this.collectFPS();
        
        // Collect memory usage
        this.collectMemoryUsage();
        
        // Collect CPU usage (estimated)
        this.collectCPUUsage();
        
        // Update history
        this.updateHistory();
    }
    
    collectFPS() {
        if (!this.lastFrameTime) {
            this.lastFrameTime = performance.now();
            this.frameCount = 0;
            return;
        }
        
        const now = performance.now();
        const delta = now - this.lastFrameTime;
        
        if (delta >= 1000) {
            this.metrics.fps = Math.round((this.frameCount * 1000) / delta);
            this.frameCount = 0;
            this.lastFrameTime = now;
        } else {
            this.frameCount++;
        }
    }
    
    collectMemoryUsage() {
        if (this.memoryAPISupported) {
            const memory = performance.memory;
            this.metrics.memoryUsage = {
                used: Math.round((memory.usedJSHeapSize / 1024 / 1024) * 100) / 100,
                total: Math.round((memory.totalJSHeapSize / 1024 / 1024) * 100) / 100,
                limit: Math.round((memory.jsHeapSizeLimit / 1024 / 1024) * 100) / 100,
                percentage: Math.round((memory.usedJSHeapSize / memory.jsHeapSizeLimit) * 100)
            };
        }
    }
    
    collectCPUUsage() {
        // Estimate CPU usage based on frame timing
        const start = performance.now();
        
        // Perform a small computational task
        let sum = 0;
        for (let i = 0; i < 10000; i++) {
            sum += Math.random();
        }
        
        const end = performance.now();
        const executionTime = end - start;
        
        // Estimate CPU usage (this is a rough approximation)
        this.metrics.cpuUsage = Math.min(100, Math.round(executionTime * 10));
    }
    
    updateHistory() {
        const maxHistoryLength = 60; // Keep 60 seconds of history
        
        Object.keys(this.history).forEach(metric => {
            if (this.metrics[metric] !== undefined) {
                this.history[metric].push(this.metrics[metric]);
                
                if (this.history[metric].length > maxHistoryLength) {
                    this.history[metric].shift();
                }
            }
        });
    }
    
    analyzePerformance() {
        const issues = [];
        
        // Check FPS
        if (this.metrics.fps < this.thresholds.fps.critical) {
            issues.push({
                type: 'critical',
                metric: 'fps',
                message: `FPS crítico: ${this.metrics.fps}`,
                suggestion: 'Reduza a qualidade do vídeo ou feche outras aplicações'
            });
        } else if (this.metrics.fps < this.thresholds.fps.warning) {
            issues.push({
                type: 'warning',
                metric: 'fps',
                message: `FPS baixo: ${this.metrics.fps}`,
                suggestion: 'Considere reduzir a resolução ou taxa de quadros'
            });
        }
        
        // Check latency
        if (this.metrics.latency > this.thresholds.latency.critical) {
            issues.push({
                type: 'critical',
                metric: 'latency',
                message: `Latência alta: ${this.metrics.latency}ms`,
                suggestion: 'Verifique sua conexão de rede'
            });
        } else if (this.metrics.latency > this.thresholds.latency.warning) {
            issues.push({
                type: 'warning',
                metric: 'latency',
                message: `Latência elevada: ${this.metrics.latency}ms`,
                suggestion: 'Considere usar uma conexão com fio'
            });
        }
        
        // Check memory usage
        if (this.metrics.memoryUsage && this.metrics.memoryUsage.percentage > this.thresholds.memoryUsage.critical) {
            issues.push({
                type: 'critical',
                metric: 'memory',
                message: `Uso de memória crítico: ${this.metrics.memoryUsage.percentage}%`,
                suggestion: 'Feche outras abas ou aplicações'
            });
        } else if (this.metrics.memoryUsage && this.metrics.memoryUsage.percentage > this.thresholds.memoryUsage.warning) {
            issues.push({
                type: 'warning',
                metric: 'memory',
                message: `Uso de memória alto: ${this.metrics.memoryUsage.percentage}%`,
                suggestion: 'Monitore o uso de memória'
            });
        }
        
        this.currentIssues = issues;
    }
    
    getOptimizationSuggestions() {
        const suggestions = [];
        
        // Based on current metrics, suggest optimizations
        if (this.metrics.fps < 30) {
            suggestions.push({
                category: 'video',
                title: 'Reduzir Qualidade de Vídeo',
                description: 'Diminua a resolução ou taxa de quadros para melhorar a performance',
                impact: 'high',
                difficulty: 'easy'
            });
        }
        
        if (this.metrics.latency > 50) {
            suggestions.push({
                category: 'network',
                title: 'Otimizar Conexão de Rede',
                description: 'Use conexão cabeada ou melhore a qualidade do Wi-Fi',
                impact: 'high',
                difficulty: 'medium'
            });
        }
        
        if (this.metrics.memoryUsage && this.metrics.memoryUsage.percentage > 70) {
            suggestions.push({
                category: 'system',
                title: 'Liberar Memória',
                description: 'Feche aplicações desnecessárias para liberar RAM',
                impact: 'medium',
                difficulty: 'easy'
            });
        }
        
        if (this.metrics.cpuUsage > 80) {
            suggestions.push({
                category: 'system',
                title: 'Reduzir Carga de CPU',
                description: 'Feche processos em segundo plano ou reduza configurações',
                impact: 'high',
                difficulty: 'medium'
            });
        }
        
        return suggestions;
    }
    
    updateWebRTCStats(stats) {
        // Update metrics from WebRTC stats
        stats.forEach(report => {
            if (report.type === 'inbound-rtp' && report.mediaType === 'video') {
                // Update bitrate
                const now = Date.now();
                const bytesDelta = report.bytesReceived - (this.lastBytesReceived || 0);
                const timeDelta = now - (this.lastStatsTime || now);
                
                if (timeDelta > 0) {
                    this.metrics.bitrate = Math.round((bytesDelta * 8 * 1000) / (timeDelta * 1024)); // Kbps
                }
                
                this.lastBytesReceived = report.bytesReceived;
                this.lastStatsTime = now;
                
                // Update dropped frames
                this.metrics.droppedFrames = report.framesDropped || 0;
                
                // Update jitter
                this.metrics.jitter = report.jitter || 0;
            }
            
            if (report.type === 'remote-inbound-rtp') {
                // Update latency (RTT)
                this.metrics.latency = Math.round((report.roundTripTime || 0) * 1000);
            }
        });
    }
    
    getPerformanceScore() {
        let score = 100;
        
        // Deduct points based on issues
        if (this.metrics.fps < 30) score -= 20;
        if (this.metrics.fps < 15) score -= 30;
        
        if (this.metrics.latency > 100) score -= 15;
        if (this.metrics.latency > 200) score -= 25;
        
        if (this.metrics.memoryUsage && this.metrics.memoryUsage.percentage > 80) score -= 10;
        if (this.metrics.memoryUsage && this.metrics.memoryUsage.percentage > 95) score -= 20;
        
        if (this.metrics.droppedFrames > 5) score -= 10;
        if (this.metrics.droppedFrames > 10) score -= 20;
        
        return Math.max(0, score);
    }
    
    getPerformanceGrade() {
        const score = this.getPerformanceScore();
        
        if (score >= 90) return { grade: 'A', color: '#10b981', description: 'Excelente' };
        if (score >= 80) return { grade: 'B', color: '#3b82f6', description: 'Bom' };
        if (score >= 70) return { grade: 'C', color: '#f59e0b', description: 'Regular' };
        if (score >= 60) return { grade: 'D', color: '#ef4444', description: 'Ruim' };
        return { grade: 'F', color: '#dc2626', description: 'Crítico' };
    }
    
    exportMetrics() {
        return {
            timestamp: Date.now(),
            metrics: { ...this.metrics },
            history: { ...this.history },
            issues: this.currentIssues || [],
            suggestions: this.getOptimizationSuggestions(),
            score: this.getPerformanceScore(),
            grade: this.getPerformanceGrade()
        };
    }
    
    onMetricsUpdate(callback) {
        const id = Date.now() + Math.random();
        this.callbacks.set(id, callback);
        return id;
    }
    
    offMetricsUpdate(id) {
        this.callbacks.delete(id);
    }
    
    notifyCallbacks() {
        const data = this.exportMetrics();
        this.callbacks.forEach(callback => {
            try {
                callback(data);
            } catch (error) {
                console.error('Error in performance callback:', error);
            }
        });
    }
    
    // Adaptive quality management
    getAdaptiveQualitySettings() {
        const settings = {
            resolution: '1920x1080',
            framerate: 30,
            bitrate: 5000,
            quality: 'high'
        };
        
        // Adapt based on performance
        if (this.metrics.fps < 20 || this.metrics.cpuUsage > 80) {
            settings.resolution = '1280x720';
            settings.framerate = 24;
            settings.bitrate = 3000;
            settings.quality = 'medium';
        }
        
        if (this.metrics.fps < 15 || this.metrics.cpuUsage > 90) {
            settings.resolution = '854x480';
            settings.framerate = 20;
            settings.bitrate = 1500;
            settings.quality = 'low';
        }
        
        if (this.metrics.latency > 200) {
            settings.framerate = Math.max(15, settings.framerate - 5);
            settings.bitrate = Math.max(1000, settings.bitrate * 0.8);
        }
        
        return settings;
    }
}

// Export for use in main application
window.PerformanceMonitor = PerformanceMonitor;

