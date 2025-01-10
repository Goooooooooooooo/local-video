class SimpleAlert {
    private alertContainer: HTMLElement | null;

    constructor() {
        this.alertContainer = null;
        this.init();
    }

    private init(): void {
        // 创建容器元素
        this.alertContainer = document.createElement('div');
        this.alertContainer.style.position = 'fixed';
        this.alertContainer.style.top = '20px';
        this.alertContainer.style.left = '50%';
        this.alertContainer.style.transform = 'translateX(-50%)';
        this.alertContainer.style.zIndex = '10000';
        this.alertContainer.style.maxWidth = '500px'; // 调整初始宽度
        this.alertContainer.style.display = 'flex';
        this.alertContainer.style.flexDirection = 'column';
        this.alertContainer.style.gap = '15px';
        document.body.appendChild(this.alertContainer);
    }

    /**
     * 成功提示
     * @param message 消息
     * @param options 自动关闭时间 duration
     */
    public showSuccess(message: string, options: { duration?: number } = {}): void {
        this.show(message, { type: 'success', ...options });
    }

    /**
     * 警告提示
     * @param message 消息
     * @param options 自动关闭时间 duration
     */
    public showWarning(message: string, options: { duration?: number } = {}): void {
        this.show(message, { type: 'warning', ...options });
    }

    /**
     * 错误提示
     * @param message 消息
     * @param options 自动关闭时间 duration
     */
    public showError(message: string, options: { duration?: number } = {}): void {
        this.show(message, { type: 'error', ...options });
    }

    public show(message: string, options: { type?: string; duration?: number } = {}): void {
        const { type = 'info', duration = 0 } = options;

        // 创建 alert 元素
        const alertBox = document.createElement('div');
        alertBox.style.display = 'flex';
        alertBox.style.alignItems = 'center';
        alertBox.style.justifyContent = 'center';
        alertBox.style.flexDirection = 'column';
        alertBox.style.gap = '10px';
        alertBox.style.padding = '25px';
        alertBox.style.borderRadius = '12px';
        alertBox.style.boxShadow = '0 12px 24px rgba(0, 0, 0, 0.2)';
        alertBox.style.fontSize = '18px';
        alertBox.style.color = this.getTextColor(type);
        alertBox.style.backgroundColor = this.getBackgroundColor(type);
        alertBox.style.opacity = '0';
        alertBox.style.transform = 'translateY(-20px) scale(0.9)';
        alertBox.style.transition = 'opacity 0.4s, transform 0.4s';

        // 图标
        const icon = document.createElement('div');
        icon.innerHTML = this.getIcon(type);
        icon.style.fontSize = '40px';
        icon.style.color = this.getIconColor(type);

        // 文本
        const text = document.createElement('div');
        text.innerHTML = message.replace(/\n/g, '<br>'); // 支持换行符
        text.style.textAlign = 'center';
        text.style.lineHeight = '1.6';

        // 关闭按钮
        const closeButton = document.createElement('button');
        closeButton.textContent = '×';
        closeButton.style.position = 'absolute';
        closeButton.style.top = '10px';
        closeButton.style.right = '10px';
        closeButton.style.border = 'none';
        closeButton.style.background = 'transparent';
        closeButton.style.fontSize = '25px';
        closeButton.style.cursor = 'pointer';
        closeButton.addEventListener('click', () => {
            alertBox.style.opacity = '0';
            alertBox.style.transform = 'translateY(-20px) scale(0.9)';
            setTimeout(() => alertBox.remove(), 400); // 动画结束后移除
        });

        alertBox.appendChild(icon);
        alertBox.appendChild(text);
        alertBox.appendChild(closeButton);
        this.alertContainer?.appendChild(alertBox);

        // 渐入效果
        requestAnimationFrame(() => {
            alertBox.style.opacity = '1';
            alertBox.style.transform = 'translateY(0) scale(1)';
        });

        // 自动移除
        if (duration > 0) {
            console.log(duration);
            setTimeout(() => {
                alertBox.style.opacity = '0';
                alertBox.style.transform = 'translateY(-20px) scale(0.9)';
                setTimeout(() => alertBox.remove(), 400); // 动画结束后移除
            }, duration);
        }
    }

    private getBackgroundColor(type: string): string {
        switch (type) {
            case 'success': return '#dff0d8';
            case 'error': return '#f2dede';
            case 'warning': return '#fcf8e3';
            default: return '#d9edf7'; // info
        }
    }

    private getTextColor(type: string): string {
        switch (type) {
            case 'success': return '#3c763d';
            case 'error': return '#a94442';
            case 'warning': return '#8a6d3b';
            default: return '#31708f'; // info
        }
    }

    private getIconColor(type: string): string {
        return this.getTextColor(type);
    }

    private getIcon(type: string): string {
        switch (type) {
            case 'success': return '✅';
            case 'error': return '❌';
            case 'warning': return '⚠️';
            default: return 'ℹ️'; // info
        }
    }
}

export default SimpleAlert;