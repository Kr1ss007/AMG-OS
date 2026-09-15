export class ZenGlanceParent extends JSWindowActorParent {
  constructor() {
    super();
  }

  async receiveMessage(message) {
    switch (message.name) {
      case 'ZenGlance:GetActivationMethod': {
        return Services.prefs.getStringPref('zen.glance.activation-method', 'ctrl');
      }
      case 'ZenGlance:GetHoverActivationDelay': {
        return Services.prefs.getIntPref('zen.glance.hold-duration', 500);
      }
      case 'ZenGlance:OpenGlance': {
        this.openGlance(this.browsingContext.topChromeWindow, message.data);
        break;
      }
      case 'ZenGlance:CloseGlance': {
        const params = {
          onTabClose: true,
          ...message.data,
        };
        this.browsingContext.topChromeWindow.gZenGlanceManager.closeGlance(params);
        break;
      }
      default:
        console.warn(`[glance]: Unknown message: ${message.name}`);
    }
  }

  openGlance(window, data) {
    window.gZenGlanceManager.openGlance(data);
  }
}
