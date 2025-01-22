class SimpleAlert {
    private static instance: SimpleAlert;
    private alertContainer: HTMLElement | null;
    private alertConfrimContainer: HTMLElement | null;

    constructor() {
        this.alertContainer = null;
        this.alertConfrimContainer = null;
    }

    public static getInstance(): SimpleAlert {
        if (!SimpleAlert.instance) {
            SimpleAlert.instance = new SimpleAlert();
        }
        return SimpleAlert.instance;
    }

    private init(option: {center: boolean} = {center: false}): HTMLElement {
        // 创建容器元素
        let _alertContainer = document.createElement('div');
        _alertContainer.style.position = 'fixed';

        if (option && option.center) {
            _alertContainer.style.left = '50%';
            _alertContainer.style.top = '50%';
            _alertContainer.style.transform = 'translate(-50%, -50%)';
            _alertContainer.style.width = '400px'; // 调整宽度
        } else {
            _alertContainer.style.top = '20px';
            _alertContainer.style.left = '50%';
            _alertContainer.style.transform = 'translateX(-50%)';
            _alertContainer.style.minWidth = '300px'; // 调整初始宽度
            _alertContainer.style.maxWidth = '500px'; // 调整初始宽度
        }

        _alertContainer.style.zIndex = '10000';

        _alertContainer.style.display = 'flex';
        _alertContainer.style.flexDirection = 'column';
        _alertContainer.style.gap = '15px';

        document.body.appendChild(_alertContainer);
        return _alertContainer;
    }

    /**
     * 提示
     * @param message 消息
     * @param options 自动关闭时间 duration
     */
    public info(message: string, options: { duration?: number } = {}): void {
        this.show(message, { type: 'info', ...options });
    }

    /**
     * 成功提示
     * @param message 消息
     * @param options 自动关闭时间 duration
     */
    public success(message: string, options: { duration?: number } = {}): void {
        this.show(message, { type: 'success', ...options });
    }

    /**
     * 警告提示
     * @param message 消息
     * @param options 自动关闭时间 duration
     */
    public warn(message: string, options: { duration?: number } = {}): void {
        this.show(message, { type: 'warning', ...options });
    }

    /**
     * 错误提示
     * @param message 消息
     * @param options 自动关闭时间 duration
     */
    public error(message: string, options: { duration?: number } = {}): void {
        this.show(message, { type: 'error', ...options });
    }

    public show(message: string, options: { type?: string; duration?: number } = {}): void {
        if (!this.alertContainer) {
            this.alertContainer = this.init();
            // console.log('alertContainer 不存在，重新创建');
        }
        const { type = 'info', duration = 0 } = options;

        // 创建 alert 元素
        const alertBox = document.createElement('div');
        alertBox.style.display = 'flex';
        alertBox.style.alignItems = 'center';
        alertBox.style.justifyContent = 'center';
        alertBox.style.flexDirection = 'column';
        alertBox.style.gap = '15px';
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
            this.removeAlert(alertBox);
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
            // console.log(duration);
            setTimeout(() => {
                this.removeAlert(alertBox);
            }, duration);
        }
    }

    /**
     * 显示确认弹窗，并允许自定义内容
     * @param message 提示信息
     * @param customContent 自定义内容，可以是 HTML 字符串
     * @param callback 用户选择回调
     */
    public confirm(message: string, customContent: string | null = null, callback: (confirmed: boolean, isChecked: boolean) => void): void {
        if (!this.alertConfrimContainer) {
            // console.log('alertContainer 不存在，重新创建');
            this.alertConfrimContainer = this.init({center: true});
        }

        // 创建背景蒙版
        const overlay = document.createElement('div');
        overlay.style.position = 'fixed';
        overlay.style.top = '0';
        overlay.style.left = '0';
        overlay.style.width = '100%';
        overlay.style.height = '100%';
        overlay.style.backgroundColor = 'rgba(0, 0, 0, 0.5)'; // 半透明黑色背景
        overlay.style.zIndex = '9999'; // 确保蒙版位于弹窗下方
        overlay.addEventListener('click', () => { // 点击背景关闭弹窗
            callback(false, false);
            this.removeAlert(confirmBox, overlay);
        });
        document.body.appendChild(overlay);

        // 创建确认弹窗
        const confirmBox = document.createElement('div');
        confirmBox.style.display = 'flex';
        confirmBox.style.alignItems = 'center';
        confirmBox.style.justifyContent = 'center';
        confirmBox.style.flexDirection = 'column';
        confirmBox.style.gap = '10px';
        confirmBox.style.padding = '25px';
        confirmBox.style.borderRadius = '12px';
        confirmBox.style.boxShadow = '0 12px 24px rgba(0, 0, 0, 0.2)';
        confirmBox.style.fontSize = '18px';
        confirmBox.style.backgroundColor = '#fff';
        confirmBox.style.color = '#000';
        confirmBox.style.opacity = '0';
        confirmBox.style.transform = 'translateY(-20px) scale(0.9)';
        confirmBox.style.transition = 'opacity 0.4s, transform 0.4s';

        const text = document.createElement('div');
        text.innerHTML = message;
        text.style.textAlign = 'center';
        text.style.lineHeight = '1.6';

        // 自定义内容
        const customContentDiv = document.createElement('div');
        if (customContent) {
            customContentDiv.innerHTML = customContent;
        }

        // 获取复选框元素（如果存在）
        let checkbox: HTMLInputElement | null = null;
        if (customContent && customContent.includes('<input type="checkbox">')) {
            checkbox = customContentDiv.querySelector('input[type="checkbox"]');
        }

        const buttonsContainer = document.createElement('div');
        buttonsContainer.style.display = 'flex';
        buttonsContainer.style.gap = '20px';

        // 确定按钮
        const confirmButton = document.createElement('button');
        confirmButton.textContent = '确定';
        confirmButton.style.padding = '10px 20px';
        confirmButton.style.fontSize = '16px';
        confirmButton.style.backgroundColor = '#4CAF50';
        confirmButton.style.color = '#fff';
        confirmButton.style.border = 'none';
        confirmButton.style.borderRadius = '5px';
        confirmButton.style.cursor = 'pointer';
        confirmButton.addEventListener('click', () => {
            const isChecked = checkbox ? checkbox.checked : false;
            callback(true, isChecked);
            this.removeAlert(confirmBox, overlay);
        });

        // 取消按钮
        const cancelButton = document.createElement('button');
        cancelButton.textContent = '取消';
        cancelButton.style.padding = '10px 20px';
        cancelButton.style.fontSize = '16px';
        cancelButton.style.backgroundColor = '#f44336';
        cancelButton.style.color = '#fff';
        cancelButton.style.border = 'none';
        cancelButton.style.borderRadius = '5px';
        cancelButton.style.cursor = 'pointer';
        cancelButton.addEventListener('click', () => {
            callback(false, false); // 取消时，checkbox 未选中
            this.removeAlert(confirmBox, overlay);
        });

        buttonsContainer.appendChild(confirmButton);
        buttonsContainer.appendChild(cancelButton);

        confirmBox.appendChild(text);
        confirmBox.appendChild(customContentDiv); // 插入自定义内容
        confirmBox.appendChild(buttonsContainer);
        this.alertConfrimContainer?.appendChild(confirmBox);

        // 渐入效果
        requestAnimationFrame(() => {
            confirmBox.style.opacity = '1';
            confirmBox.style.transform = 'translateY(0) scale(1)';
        });
    }


    private removeAlert(alertBox: HTMLElement, overlay?: HTMLElement): void {
        alertBox.style.opacity = '0';
        alertBox.style.transform = 'translateY(-20px) scale(0.9)';
        if (overlay) {
            overlay.style.opacity = '0'; // 隐藏蒙版
        }
        setTimeout(() => {
            alertBox.remove();
            overlay?.remove(); // 移除蒙版
            // 只有当容器内没有其他弹窗时才移除容器
            if (this.alertContainer && !this.alertContainer.hasChildNodes()) {
                document.body.removeChild(this.alertContainer);
                this.alertContainer = null;
            }
            if (this.alertConfrimContainer && !this.alertConfrimContainer.hasChildNodes()) {
                document.body.removeChild(this.alertConfrimContainer);
                this.alertConfrimContainer = null;
            }
        }, 400);
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

const simpleAlert = new SimpleAlert();
export default simpleAlert;